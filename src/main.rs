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

#![allow(clippy::disallowed_types)]

use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use color_eyre::eyre::WrapErr;
use similar::TextDiff;
use tokio::net::TcpListener;

mod logging;

/// The main server entrypoint for the API.
fn main() -> color_eyre::Result<ExitCode>
{
	color_eyre::install()?;

	match Args::parse().action {
		Action::Serve { ip_address, port, config } => {
			serve(ip_address, port, config)?;
		}

		Action::GenerateSchema { check } => {
			return generate_schema(check);
		}
	}

	Ok(ExitCode::SUCCESS)
}

#[tokio::main]
async fn serve(
	ip_address: IpAddr,
	port: u16,
	config: cs2kz_api::runtime::Config,
) -> color_eyre::Result<()>
{
	cs2kz_api::runtime::panic_hook::install();

	let _guard = logging::init().context("initialize logging")?;

	let tcp_listener = TcpListener::bind(SocketAddr::new(ip_address, port))
		.await
		.context("bind tcp listener")?;

	let server = cs2kz_api::server(config).await.context("run server")?;

	tracing::info!("listening on {}", tcp_listener.local_addr()?);

	axum::serve(tcp_listener, server).await.context("run axum")
}

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
enum Action
{
	/// Serve the API.
	Serve
	{
		/// The IP address you want the API to listen on.
		#[clap(default_value = "127.0.0.1")]
		#[arg(long = "ip")]
		ip_address: IpAddr,

		/// The port you want the API to listen on.
		#[clap(default_value = "42069")]
		#[arg(long)]
		port: u16,

		#[allow(clippy::missing_docs_in_private_items)]
		#[command(flatten)]
		config: cs2kz_api::runtime::Config,
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
