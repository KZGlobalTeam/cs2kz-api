//! The CS2KZ API

#![allow(clippy::redundant_closure, clippy::new_without_default)]
#![warn(
	clippy::absolute_paths,
	clippy::as_underscore,
	clippy::cognitive_complexity,
	clippy::collection_is_never_read,
	clippy::dbg_macro,
	clippy::future_not_send,
	clippy::todo
)]
#![deny(
	missing_debug_implementations,
	missing_docs,
	clippy::missing_docs_in_private_items,
	rustdoc::broken_intra_doc_links,
	clippy::perf,
	clippy::bool_comparison,
	clippy::bool_to_int_with_if,
	clippy::cast_possible_truncation,
	clippy::clone_on_ref_ptr,
	clippy::ignored_unit_patterns,
	clippy::unimplemented
)]

use std::future::Future;

use axum::routing::{get, IntoMakeService};
use axum::serve::Serve;
use axum::Router;
use eyre::Context;
use itertools::Itertools;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{debug, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use self::security::Security;

mod error;
pub use error::{Error, Result};

mod config;
pub use config::Config;

mod state;
pub use state::State;

#[cfg(test)]
mod test;

#[cfg(test)]
pub(crate) use cs2kz_api_macros::test;

mod responses;
mod parameters;
mod middleware;
mod sqlx;
mod workshop;
mod security;
mod serde;
mod time;
mod id;

mod players;
mod maps;
mod servers;
mod jumpstats;
mod records;
mod bans;
mod game_sessions;
mod auth;
mod admins;
mod plugin;

#[derive(OpenApi)]
#[rustfmt::skip]
#[openapi(
  info(
    title = "CS2KZ API",
    description = "Source Code available on [GitHub](https://github.com/KZGlobalTeam/cs2kz-api).",
    license(
      name = "Licensed under the GPLv3",
      url = "https://www.gnu.org/licenses/gpl-3.0",
    ),
  ),
  modifiers(&Security),
  paths(
    players::handlers::root::get,
    players::handlers::root::post,
    players::handlers::by_identifier::get,
    players::handlers::by_identifier::patch,
    players::handlers::preferences::get,
    players::handlers::preferences::put,

    maps::handlers::root::get,
    maps::handlers::root::put,
    maps::handlers::by_identifier::get,
    maps::handlers::by_identifier::patch,

    servers::handlers::root::get,
    servers::handlers::root::post,
    servers::handlers::by_identifier::get,
    servers::handlers::by_identifier::patch,
    servers::handlers::key::generate_temp,
    servers::handlers::key::put_perma,
    servers::handlers::key::delete_perma,

    jumpstats::handlers::root::get,
    jumpstats::handlers::root::post,
    jumpstats::handlers::by_id::get,
    jumpstats::handlers::replays::get,

    records::handlers::root::get,
    records::handlers::root::post,
    records::handlers::top::get,
    records::handlers::by_id::get,
    records::handlers::replays::get,

    bans::handlers::root::get,
    bans::handlers::root::post,
    bans::handlers::by_id::get,
    bans::handlers::by_id::patch,
    bans::handlers::by_id::delete,

    game_sessions::handlers::by_id::get,

    auth::handlers::login,
    auth::handlers::logout,
    auth::handlers::callback,

    admins::handlers::root::get,
    admins::handlers::by_id::get,
    admins::handlers::by_id::put,

    plugin::handlers::versions::get,
    plugin::handlers::versions::post,
  ),
  components(
    schemas(
      cs2kz::SteamID,
      cs2kz::Mode,
      cs2kz::Style,
      cs2kz::Tier,
      cs2kz::JumpType,
      cs2kz::PlayerIdentifier,
      cs2kz::MapIdentifier,
      cs2kz::ServerIdentifier,
      cs2kz::GlobalStatus,
      cs2kz::RankedStatus,

      parameters::Offset,
      parameters::Limit,

      time::Seconds,

      responses::Object,

      players::models::Player,
      players::models::NewPlayer,
      players::models::PlayerUpdate,

      maps::models::FullMap,
      maps::models::Course,
      maps::models::Filter,
      maps::models::NewMap,
      maps::models::NewCourse,
      maps::models::NewFilter,
      maps::models::CreatedMap,
      maps::models::MapUpdate,
      maps::models::CourseUpdate,
      maps::models::FilterUpdate,
      maps::models::MapInfo,
      maps::models::CourseInfo,

      servers::models::Server,
      servers::models::NewServer,
      servers::models::CreatedServer,
      servers::models::ServerUpdate,
      servers::models::RefreshKeyRequest,
      servers::models::RefreshKey,
      servers::models::ServerInfo,

      jumpstats::models::Jumpstat,
      jumpstats::models::NewJumpstat,
      jumpstats::models::CreatedJumpstat,

      records::models::Record,
      records::models::BhopStats,
      records::models::NewRecord,
      records::models::CreatedRecord,

      bans::models::Ban,
      bans::models::BanReason,
      bans::models::Unban,
      bans::models::NewBan,
      bans::models::CreatedBan,
      bans::models::BanUpdate,
      bans::models::NewUnban,
      bans::models::CreatedUnban,

      game_sessions::models::GameSession,
      game_sessions::models::TimeSpent,

      admins::models::Admin,
      admins::models::AdminUpdate,

      plugin::models::PluginVersion,
      plugin::models::NewPluginVersion,
      plugin::models::CreatedPluginVersion,
    ),
  ),
)]
#[allow(missing_docs, missing_debug_implementations)]
pub struct API;

impl API {
	/// Run the API.
	pub async fn run(config: Config) -> eyre::Result<()> {
		Self::server(config)
			.await
			.context("build http server")?
			.with_graceful_shutdown(sigint())
			.await
			.context("run http server")
	}

	/// Run the API, until the given `until` future completes.
	pub async fn run_until<Until>(config: Config, until: Until) -> eyre::Result<()>
	where
		Until: Future<Output = ()> + Send + 'static,
	{
		Self::server(config)
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

	/// Creates a hyper server that will serve the API.
	async fn server(config: Config) -> eyre::Result<Serve<IntoMakeService<Router>, Router>> {
		info!(target: "audit_log", ?config, "API starting up");

		let tcp_listener = TcpListener::bind(config.socket_addr())
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

		let openapi = Self::openapi();

		openapi
			.paths
			.paths
			.iter()
			.map(|(path, handler)| {
				let methods = handler
					.operations
					.keys()
					.map(|method| format!("{method:?}").to_uppercase())
					.join(", ");

				format!("{path} [{methods}]")
			})
			.for_each(|route| debug!("registering route: {route}"));

		let api_service = Router::new()
			.route("/", get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
			.nest("/players", players::router(state))
			.nest("/maps", maps::router(state))
			.nest("/servers", servers::router(state))
			.nest("/jumpstats", jumpstats::router(state))
			.nest("/records", records::router(state))
			.nest("/bans", bans::router(state))
			.nest("/sessions", game_sessions::router(state))
			.nest("/auth", auth::router(state))
			.nest("/admins", admins::router(state))
			.nest("/plugin", plugin::router(state))
			.layer(middleware::logging::layer!())
			.merge(SwaggerUi::new("/docs/swagger-ui").url("/docs/openapi.json", openapi))
			.into_make_service();

		let address = tcp_listener.local_addr().context("get tcp addr")?;

		info! {
			target: "audit_log",
			%address,
			prod = cfg!(feature = "production"),
			"listening for requests",
		};

		Ok(axum::serve(tcp_listener, api_service))
	}

	/// Generates a JSON version of the OpenAPI spec.
	pub fn spec() -> String {
		Self::openapi().to_pretty_json().expect("spec is valid")
	}
}

/// Waits for and handles potential errors from SIGINT (ctrl+c) from the OS.
async fn sigint() {
	match signal::ctrl_c().await {
		Ok(()) => tracing::warn!(target: "audit_log", "received SIGINT; shutting down..."),
		Err(err) => tracing::error!(target: "audit_log", "failed to receive SIGINT: {err}"),
	}
}
