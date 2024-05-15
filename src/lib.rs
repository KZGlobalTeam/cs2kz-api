#![doc = include_str!("../README.md")]

use std::future::Future;

use anyhow::Context;
use axum::routing::{get, IntoMakeService};
use axum::serve::Serve;
use axum::Router;
use itertools::Itertools;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{debug, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use self::openapi::security::Security;

mod error;

#[doc(inline)]
pub use error::{Error, Result};

mod config;

#[doc(inline)]
pub use config::Config;

mod state;

#[doc(inline)]
pub(crate) use state::State;

#[cfg(test)]
mod test;

#[cfg(test)]
#[doc(inline)]
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

#[derive(Debug, Clone, Copy, OpenApi)]
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
    players::handlers::steam::get,
    players::handlers::preferences::get,

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

    authentication::handlers::login,
    authentication::handlers::logout,
    authentication::handlers::callback,

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
      cs2kz::CourseIdentifier,
      cs2kz::ServerIdentifier,
      cs2kz::GlobalStatus,
      cs2kz::RankedStatus,

      openapi::parameters::Offset,
      openapi::parameters::Limit,
      openapi::parameters::SortingOrder,
      openapi::responses::Object,

      time::Seconds,

      steam::workshop::WorkshopID,

      players::Player,
      players::NewPlayer,
      players::PlayerUpdate,

      maps::FullMap,
      maps::MapID,
      maps::Course,
      maps::CourseID,
      maps::Filter,
      maps::FilterID,
      maps::NewMap,
      maps::NewCourse,
      maps::NewFilter,
      maps::CreatedMap,
      maps::MapUpdate,
      maps::CourseUpdate,
      maps::FilterUpdate,
      maps::MapInfo,
      maps::CourseInfo,

      servers::Server,
      servers::ServerID,
      servers::NewServer,
      servers::CreatedServer,
      servers::ServerUpdate,
      servers::RefreshKeyRequest,
      servers::RefreshKey,
      servers::ServerInfo,

      jumpstats::Jumpstat,
      jumpstats::JumpstatID,
      jumpstats::NewJumpstat,
      jumpstats::CreatedJumpstat,

      records::Record,
      records::RecordID,
      records::BhopStats,
      records::NewRecord,
      records::CreatedRecord,
      records::handlers::root::SortRecordsBy,

      bans::Ban,
      bans::BanID,
      bans::BanReason,
      bans::Unban,
      bans::UnbanID,
      bans::NewBan,
      bans::CreatedBan,
      bans::BanUpdate,
      bans::NewUnban,
      bans::CreatedUnban,

      game_sessions::GameSession,
      game_sessions::GameSessionID,
      game_sessions::TimeSpent,

      admins::Admin,
      admins::AdminUpdate,

      plugin::PluginVersion,
      plugin::PluginVersionID,
      plugin::NewPluginVersion,
      plugin::CreatedPluginVersion,
    ),
  ),
)]
#[allow(missing_docs)]
pub struct API;

impl API {
	/// Run the API.
	pub async fn run(config: Config) -> anyhow::Result<()> {
		Self::server(config)
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
	async fn server(config: Config) -> anyhow::Result<Serve<IntoMakeService<Router>, Router>> {
		info!(target: "audit_log", ?config, "API starting up");

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
			.nest("/auth", authentication::router(state))
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
