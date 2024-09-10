//! CS2KZ API - the core infrastructure for CS2KZ.
//! Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program. If not, see https://www.gnu.org/licenses.

#![expect(clippy::disallowed_types)]

use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use color_eyre::eyre::WrapErr;
use similar::TextDiff;
use tap::Tap;
use tokio::net::TcpListener;

mod tracing;

/// The main server entrypoint for the API.
fn main() -> color_eyre::Result<ExitCode>
{
	color_eyre::install()?;

	match Args::parse().action {
		Action::Serve { ip_address, port, config } => {
			if dotenvy::dotenv().is_err() {
				eprintln!("WARNING: no `.env` file found");
			}

			let mut config = cs2kz_api::runtime::Config::load(config).context("load config")?;

			if let Some(ip) = ip_address {
				config.http.listen_addr = ip;
			}

			if let Some(port) = port {
				config.http.listen_port = port;
			}

			let mut runtime = tokio::runtime::Builder::new_multi_thread().tap_mut(|builder| {
				builder.enable_all();
			});

			if let Some(worker_threads) = config.runtime.worker_threads {
				runtime.worker_threads(worker_threads.get());
			}

			if let Some(max_blocking_threads) = config.runtime.max_blocking_threads {
				runtime.max_blocking_threads(max_blocking_threads.get());
			}

			runtime.thread_stack_size(config.runtime.thread_stack_size);

			#[cfg(all(feature = "console", tokio_unstable))]
			if config.runtime.metrics.record_poll_counts {
				runtime.enable_metrics_poll_count_histogram();
			}

			runtime
				.build()
				.context("build tokio runtime")?
				.block_on(serve(config))?;
		}

		Action::GenerateSchema { check } => {
			return generate_schema(check);
		}
	}

	Ok(ExitCode::SUCCESS)
}

/// Serves the API on the given `ip_address` and `port` with the given `config`.
async fn serve(config: cs2kz_api::runtime::Config) -> color_eyre::Result<()>
{
	cs2kz_api::runtime::panic_hook::install();

	let _tracing_guard = self::tracing::init(config.tracing).context("initalize tracing")?;

	let tcp_listener = TcpListener::bind(config.http.socket_addr())
		.await
		.context("bind tcp listener")?;

	let server = cs2kz_api::server(
		config.runtime,
		config.database,
		config.http,
		config.secrets,
		config.steam,
	)
	.await
	.context("run server")?;

	::tracing::info!("listening on {}", tcp_listener.local_addr()?);

	axum::serve(tcp_listener, server)
		.with_graceful_shutdown(cs2kz_api::runtime::signals::sigint())
		.await
		.context("run axum")
}

/// Generates the API's OpenAPI schema and either writes it to stdout, or diffs
/// it against an existing file.
fn generate_schema(check_against: Option<PathBuf>) -> color_eyre::Result<ExitCode>
{
	let schema = cs2kz_api::openapi::Schema::json();

	let Some(path) = check_against else {
		print!("{schema}");
		return Ok(ExitCode::SUCCESS);
	};

	let file = fs::read_to_string(&path).with_context(|| format!("read {path:?}"))?;
	let exit_code = TextDiff::from_lines(&file, &schema)
		.unified_diff()
		.iter_hunks()
		.fold(ExitCode::SUCCESS, |_, hunk| {
			eprintln!("{hunk}");
			ExitCode::FAILURE
		});

	Ok(exit_code)
}

/// CS2KZ API
///
/// This is the server binary that will run the API.
/// You can configure it by passing CLI flags or setting environment variables
/// as described below.
#[derive(Debug, Parser)]
struct Args
{
	/// The action you want to perform.
	#[command(subcommand)]
	action: Action,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Action
{
	/// Serve the API.
	Serve
	{
		/// The IP address you want the API to listen on.
		#[arg(long = "ip")]
		ip_address: Option<IpAddr>,

		/// The port you want the API to listen on.
		#[arg(long)]
		port: Option<u16>,

		/// Path to the configuration file to use.
		#[clap(default_value = ".config/config.toml")]
		#[arg(long)]
		config: PathBuf,
	},

	/// Generate the API's OpenAPI schema.
	GenerateSchema
	{
		/// Generate the schema and diff it against an existing file.
		///
		/// If any diff is produced, the program will terminate with a non-zero
		/// exit code.
		#[arg(long, name = "FILE")]
		check: Option<PathBuf>,
	},
}
