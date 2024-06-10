#![doc = include_str!("../README.md")]
// TODO: remove once https://github.com/tokio-rs/tracing/issues/2912 lands
#![allow(clippy::blocks_in_conditions)]

use std::fmt::Write;
use std::future::Future;
use std::net::SocketAddr;

use anyhow::Context;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::ConnectInfo;
use axum::serve::Serve;
use axum::{routing, Router};
use tokio::net::TcpListener;
use tokio::signal;

mod error;
pub use error::{Error, Result};

mod config;
pub use config::Config;

mod state;
pub(crate) use state::State;

#[cfg(test)]
mod test;

#[cfg(test)]
pub(crate) use cs2kz_api_macros::integration_test;

pub mod openapi;
pub mod middleware;
pub mod authentication;
pub mod authorization;
pub mod sqlx;
pub mod steam;
pub mod serde;
pub mod time;
pub mod make_id;
pub mod bitflags;
pub mod kz;

pub mod players;
pub mod maps;
pub mod servers;
pub mod jumpstats;
pub mod records;
pub mod bans;
pub mod game_sessions;
pub mod admins;
pub mod plugin;

/// This is nasty.
type Server = Serve<
	IntoMakeServiceWithConnectInfo<Router, SocketAddr>,
	axum::middleware::AddExtension<Router, ConnectInfo<SocketAddr>>,
>;

/// Run the API.
pub async fn run(config: Config) -> anyhow::Result<()> {
	server(config)
		.await
		.context("build http server")?
		.with_graceful_shutdown(sigint())
		.await
		.context("run http server")
}

/// Run the API, until the given `until` future completes.
pub async fn run_until<Until>(config: Config, until: Until) -> anyhow::Result<()>
where
	Until: Future<Output = ()> + Send + 'static,
{
	server(config)
		.await
		.context("build http server")?
		.with_graceful_shutdown(async move {
			tokio::select! {
				() = until => {}
				() = sigint() => {}
			}
		})
		.await
		.context("run http server")
}

/// Creates an axum server that will serve the API.
async fn server(config: Config) -> anyhow::Result<Server> {
	tracing::debug!(addr = %config.addr, "establishing TCP connection");

	let tcp_listener = TcpListener::bind(config.addr)
		.await
		.context("bind tcp socket")?;

	// NOTE: We intentionally **leak memory here**.
	//       The application is not going to do anything after axum shuts down, so
	//       there is no point in cleanup.
	let state: &'static State = State::new(config)
		.await
		.map(Box::new)
		.map(Box::leak)
		.context("initialize state")?;

	let spec = openapi::Spec::new();
	let mut routes_message = String::from("registering routes:\n");

	for (path, methods) in spec.routes() {
		writeln!(&mut routes_message, "    • {path} => [{methods}]")?;
	}

	tracing::info!("{routes_message}");
	tracing::debug!("initializing API service");

	let api_service = Router::new()
		.route("/", routing::get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
		.nest("/players", players::router(state))
		.nest("/maps", maps::router(state))
		.nest("/servers", servers::router(state))
		.nest("/jumpstats", jumpstats::router(state))
		.nest("/records", records::router(state))
		.nest("/bans", bans::router(state))
		.nest("/sessions", game_sessions::router(state))
		.nest("/auth", authentication::router(state))
		.nest("/admins", admins::router(state))
		.nest("/plugin", plugin::router(state))
		.layer(middleware::logging::layer!())
		.merge(spec.swagger_ui())
		.into_make_service_with_connect_info::<SocketAddr>();

	let addr = tcp_listener.local_addr().context("get tcp addr")?;
	tracing::info!(%addr, prod = cfg!(feature = "production"), "listening for requests");

	Ok(axum::serve(tcp_listener, api_service))
}

/// Waits for and handles potential errors from SIGINT (ctrl+c) from the OS.
#[tracing::instrument(name = "runtime::signals")]
async fn sigint() {
	let signal_result = signal::ctrl_c().await;

	if let Err(err) = signal_result {
		tracing::error!(target: "cs2kz_api::audit_log", "failed to receive SIGINT: {err}");
	} else {
		tracing::warn!(target: "cs2kz_api::audit_log", "received SIGINT; shutting down...");
	}
}
