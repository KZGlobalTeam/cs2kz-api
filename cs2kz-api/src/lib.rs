//! CS2KZ API

#![warn(missing_debug_implementations, rust_2018_idioms, clippy::style)]
#![deny(missing_docs)]

use std::fmt::Write;
use std::net::SocketAddr;

use color_eyre::eyre::Context;
use tokio::net::TcpListener;
use tracing::{debug, info};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod error;
pub use error::{Error, Result};

pub mod config;
pub use config::Config;

pub mod state;
pub use state::AppState;

/// Convenience type alias for extracing [`AppState`] from a handler function parameter.
pub type State = axum::extract::State<&'static crate::state::AppState>;

pub mod serde;
pub mod sql;
pub mod steam;

pub mod responses;
pub mod models;
pub mod jwt;

pub mod openapi;
pub use openapi::Security;

pub mod routes;
pub mod middleware;

/// Utility struct for documenting and running the API.
///
/// It is used as a sort of "namespace", because [`utoipa`] requires a struct that implements the
/// [`OpenApi`] trait in order to generate documentation.
#[rustfmt::skip]
#[derive(Debug, OpenApi)]
#[openapi(
	info(title = "CS2KZ API", license(name = "GPLv3.0", url = "https://www.gnu.org/licenses/gpl-3.0")),
	modifiers(&Security),
	components(
		schemas(
			cs2kz::SteamID,
			cs2kz::Mode,
			cs2kz::Style,
			cs2kz::Jumpstat,
			cs2kz::Tier,
			cs2kz::PlayerIdentifier<'_>,
			cs2kz::MapIdentifier<'_>,
			cs2kz::ServerIdentifier<'_>,

			models::Player,
			models::KZMap,
			models::Mapper,
			models::Course,
			models::Filter,
			models::CourseWithFilter,
			models::ServerSummary,
			models::ServerResponse,
			models::JumpstatResponse,
			models::Record,
			models::BhopStats,
			models::Ban,

			routes::players::CreatePlayerRequest,
			routes::players::UpdatePlayerRequest,
			routes::players::Session,

			routes::maps::CreateMapRequest,
			routes::maps::CreateCourse,
			routes::maps::UpdateMapRequest,
			routes::maps::CourseUpdate,
			routes::maps::FilterUpdate,
			routes::maps::CreateMapResponse,

			routes::servers::CreateServerRequest,
			routes::servers::UpdateServerRequest,
			routes::servers::CreateServerResponse,

			routes::jumpstats::CreateJumpstatRequest,
			routes::jumpstats::CreatedJumpstatResponse,

			routes::records::CreateRecordRequest,
			routes::records::CreatedRecordResponse,

			routes::auth::AuthRequest,
		),
	),

	paths(
		routes::status,

		routes::players::get_players,
		routes::players::create_player,
		routes::players::get_player_by_ident,
		routes::players::update_player,

		routes::maps::get_maps,
		routes::maps::create_map,
		routes::maps::get_map_by_ident,
		routes::maps::update_map,
		routes::maps::get_map_by_workshop_id,

		routes::servers::get_servers,
		routes::servers::create_server,
		routes::servers::get_server_by_ident,
		routes::servers::update_server,

		routes::jumpstats::get_jumpstats,
		routes::jumpstats::create_jumpstat,

		routes::records::get_records,
		routes::records::create_record,

		routes::bans::get_bans,

		routes::auth::refresh_token,
		routes::auth::steam::login,
		routes::auth::steam::callback,
	),
)]
pub struct API;

impl API {
	/// Starts an [`axum`] server to serve the API.
	#[tracing::instrument]
	pub async fn run(config: Config) -> color_eyre::Result<()> {
		let state = AppState::new(config.database_url, config.jwt_secret, config.api_url).await?;

		debug!("Initialized application state.");

		let swagger_ui = Self::swagger_ui();
		let api_service = routes::router(state)
			.merge(swagger_ui)
			.into_make_service_with_connect_info::<SocketAddr>();

		debug!("Initialized API service.");

		let tcp_listener = TcpListener::bind(&config.socket_addr)
			.await
			.context("Failed to bind TCP socket.")?;

		debug!("Bound to TCP socket on {}.", config.socket_addr);

		let socket_addr = tcp_listener
			.local_addr()
			.context("Failed to get TCP socket address.")?;

		info!("Listening on {socket_addr}...");

		let mut routes = String::from("Routes:\n");

		for route in Self::routes() {
			writeln!(&mut routes, "{s}• {route}", s = " ".repeat(16))?;
		}

		info!("{routes}");
		info!("Hosting SwaggerUI at: <http://{socket_addr}/docs/swagger-ui>");
		info!("Hosting OpenAPI spec at: <http://{socket_addr}/docs/openapi.json>");

		axum::serve(tcp_listener, api_service)
			.await
			.context("Failed to run axum server.")?;

		Ok(())
	}

	/// Creates a service hosting a SwaggerUI web page and OpenAPI doc.
	pub fn swagger_ui() -> SwaggerUi {
		SwaggerUi::new("/docs/swagger-ui").url("/docs/openapi.json", Self::openapi())
	}

	/// Creates an iterator over all public routes.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_iter().map(|(uri, path)| {
			let methods = path
				.operations
				.into_keys()
				.map(|method| format!("{method:?}").to_uppercase())
				.collect::<Vec<_>>()
				.join(", ");

			format!("{uri} [{methods}]")
		})
	}

	/// Generates an OpenAPI specification as JSON.
	pub fn spec() -> color_eyre::Result<String> {
		Self::openapi()
			.to_pretty_json()
			.context("Failed to format API spec as JSON.")
	}
}
