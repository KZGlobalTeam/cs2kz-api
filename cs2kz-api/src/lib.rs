use std::fmt::Write;
use std::net::SocketAddr;

use axum::routing::{get, patch, post};
use axum::{Router, ServiceExt};
use color_eyre::eyre::Context;
use tokio::net::TcpListener;
use tower_http::normalize_path::NormalizePathLayer;
use tower_layer::Layer;
use tracing::info;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::OpenApi;
use utoipa::{Modify, OpenApi as _};
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

pub mod error;
pub use error::{Error, Result};

pub mod logging;
pub mod database;

pub mod state;
pub mod routes;
pub mod middleware;
pub mod res;

#[rustfmt::skip]
#[derive(utoipa::OpenApi)]
#[openapi(
	info(
		title = "CS2KZ API",
		version = "0.0.0",
		license(
			name = "License: GPLv3.0",
			url = "https://www.gnu.org/licenses/gpl-3.0",
		),
	),

	paths(
		routes::health::health,

		routes::auth::refresh_token,

		routes::players::get_players,
		routes::players::get_player,
		routes::players::create_player,
		routes::players::update_player,

		routes::bans::get_bans,
		routes::bans::get_replay,
		routes::bans::create_ban,

		routes::maps::get_maps,
		routes::maps::get_map,
		routes::maps::create_map,
		routes::maps::update_map,

		routes::servers::get_servers,
		routes::servers::get_server,
		routes::servers::create_server,
		routes::servers::update_server,

		routes::records::get_records,
		routes::records::get_record,
		routes::records::get_replay,
		routes::records::create_record,
	),

	components(
		schemas(
			cs2kz::SteamID,
			cs2kz::PlayerIdentifier,
			cs2kz::MapIdentifier,
			cs2kz::ServerIdentifier,
			cs2kz::Mode,
			cs2kz::Style,
			cs2kz::Jumpstat,
			cs2kz::Tier,
			cs2kz::Runtype,

			crate::res::PlayerInfo,

			crate::res::player::Player,
			crate::routes::players::NewPlayer,
			crate::routes::players::PlayerUpdate,
			crate::routes::players::SessionData,

			crate::res::bans::Ban,
			crate::res::bans::BanReason,
			crate::routes::bans::NewBan,
			crate::routes::bans::CreatedBan,

			crate::res::maps::KZMap,
			crate::res::maps::MapCourse,
			crate::res::maps::CourseFilter,
			crate::routes::maps::NewMap,
			crate::routes::maps::Course,
			crate::routes::maps::Filter,
			crate::routes::maps::CreatedMap,
			crate::routes::maps::CreatedCourse,
			crate::routes::maps::CreatedFilter,
			crate::routes::maps::MapUpdate,
			crate::routes::maps::FilterWithCourseId,

			crate::res::servers::Server,
			crate::routes::servers::NewServer,
			crate::routes::servers::CreatedServer,
			crate::routes::servers::ServerUpdate,

			crate::res::records::Record,
			crate::res::records::RecordMap,
			crate::res::records::RecordCourse,
			crate::res::records::RecordPlayer,
			crate::res::records::RecordServer,
			crate::routes::records::NewRecord,
			crate::routes::records::BhopStats,
			crate::routes::records::CreatedRecord,
		),
	),

	modifiers(&Security),
)]
pub struct API;

pub struct Security;

impl Modify for Security {
	fn modify(&self, openapi: &mut OpenApi) {
		let Some(components) = openapi.components.as_mut() else {
			return;
		};

		components.add_security_scheme(
			"API Token",
			SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
		);
	}
}

impl API {
	/// Serves an [`axum::Router`] at the given `addr`.
	pub async fn run(router: Router, addr: SocketAddr) -> color_eyre::Result<()> {
		use axum::extract::Request as R;

		let router = NormalizePathLayer::trim_trailing_slash().layer(router);
		let service = ServiceExt::<R>::into_make_service_with_connect_info::<SocketAddr>(router);

		let tcp_listener = TcpListener::bind(addr)
			.await
			.context("Failed to bind TCP listener.")?;

		let addr = tcp_listener.local_addr()?;

		info!("Listening on {addr}.");

		let mut routes = String::from("Registering routes:\n");

		for route in API::routes() {
			writeln!(&mut routes, "\t\t\t\t\t• `{route}`")?;
		}

		info!("{routes}");
		info!("SwaggerUI hosted at: <http://{addr}/api/docs/swagger-ui>");
		info!("OpenAPI spec hosted at: <http://{addr}/api/docs/openapi.json>");

		axum::serve(tcp_listener, service)
			.await
			.context("Failed to run axum.")?;

		Ok(())
	}

	/// Creates a [`Router`] which will be used by the HTTP server.
	pub fn router(state: AppState) -> Router {
		// The state will live as long as the whole application, so leaking it is fine.
		let state: &'static AppState = Box::leak(Box::new(state));

		let public_api_router = Router::new()
			.route("/", get(routes::health::health))
			.route("/players", get(routes::players::get_players))
			.route("/players/:ident", get(routes::players::get_player))
			.route("/bans", get(routes::bans::get_bans))
			.route("/bans/:id/replay", get(routes::bans::get_replay))
			.route("/maps", get(routes::maps::get_maps))
			.route("/maps/:ident", get(routes::maps::get_map))
			.route("/servers", get(routes::servers::get_servers))
			.route("/servers/:ident", get(routes::servers::get_server))
			.route("/records", get(routes::records::get_records))
			.route("/record/:id", get(routes::records::get_record))
			.route("/record/:id/replay", get(routes::records::get_replay))
			.route("/auth/refresh_token", post(routes::auth::refresh_token))
			.with_state(state);

		let game_server_auth =
			axum::middleware::from_fn_with_state(state, middleware::auth::gameservers::auth_server);

		// These routes are to be used only by CS2 servers and require auth.
		let game_server_router = Router::new()
			.route("/players", post(routes::players::create_player))
			.route("/players/:ident", patch(routes::players::update_player))
			.route("/bans", post(routes::bans::create_ban))
			.route("/records", post(routes::records::create_record))
			.layer(game_server_auth)
			.with_state(state);

		// TODO(AlphaKeks): implement auth for this
		//
		// Ideally we use Steam for authenticating admins who are allowed to approve and
		// change maps, servers, ban players etc.

		// let map_approval_router = Router::new()
		// 	.route("/maps", post(routes::maps::create_map))
		// 	.route("/maps/:ident", patch(routes::maps::update_map))
		// 	.with_state(state);

		// let server_approval_router = Router::new()
		// 	.route("/servers", post(routes::servers::create_server))
		// 	.route("/servers/:ident", patch(routes::servers::update_server))
		// 	.with_state(state);

		let logging = axum::middleware::from_fn(middleware::logging::log_request);
		let api_router = game_server_router.merge(public_api_router).layer(logging);
		let swagger_ui = Self::swagger_ui();

		Router::new().nest("/api", api_router).merge(swagger_ui)
	}

	/// Creates an iterator over all of the API's routes.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_keys()
	}

	/// Returns a JSON version of the [OpenAPI] spec.
	///
	/// [OpenAPI]: https://www.openapis.org
	pub fn json() -> color_eyre::Result<String> {
		Self::openapi()
			.to_pretty_json()
			.context("Failed to convert API spec to JSON.")
	}

	/// Creates a [service layer] for serving an HTML page with [SwaggerUI].
	///
	/// [service layer]: https://docs.rs/tower/latest/tower/trait.Layer.html
	/// [SwaggerUI]: https://swagger.io/tools/swagger-ui
	pub fn swagger_ui() -> SwaggerUi {
		SwaggerUi::new("/api/docs/swagger-ui").url("/api/docs/openapi.json", Self::openapi())
	}
}

/// Type alias for convenience.
///
/// You can read more about axum's extractors [here].
///
/// Usually you would write a handler function like this:
///
/// ```
/// use axum::extract::State;
/// use cs2kz_api::State as AppState;
///
/// async fn handler(State(state): State<&'static AppState>) {
///     let db = state.database();
///     // ...
/// }
/// ```
///
/// To avoid all that type "boilerplate", you can use this type alias instead:
///
/// ```
/// use cs2kz_api::State;
///
/// async fn handler(state: cs2kz_api::State) {
///     let db = state.database();
///     // ...
/// }
/// ```
///
/// [here]: axum::extract
pub type State = axum::extract::State<&'static crate::AppState>;
