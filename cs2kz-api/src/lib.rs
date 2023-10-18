// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::state::AppState,
	axum::{extract::State as StateExtractor, http::Method, routing, Router},
	tower_http::{cors, cors::CorsLayer},
	utoipa::OpenApi,
	utoipa_swagger_ui::SwaggerUi,
};

pub mod error;
pub use error::{Error, Result};

pub mod logging;
pub mod routes;
pub mod state;
pub mod middleware;
pub mod responses;

/// Type alias for easy use in function signatures.
///
/// You can read more about axum's extractors
/// [here](https://docs.rs/axum/0.6.20/axum/index.html#extractors).
///
/// Usually you would write a handler function like this:
///
/// ```rust
/// use crate::State as AppState;
/// use axum::extract::State;
///
/// async fn handler(State(state): State<&'static AppState>) {
///     let db = state.database();
///     // ...
/// }
/// ```
///
/// To avoid all that type "boilerplate", you can use this type alias instead:
///
/// ```rust
/// use crate::State;
///
/// async fn handler(state: State) {
///     let db = state.database();
///     // ...
/// }
/// ```
pub type State = StateExtractor<&'static AppState>;

#[rustfmt::skip]
#[derive(OpenApi)]
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
		routes::players::create,
		routes::players::update,
		routes::records::create,
		routes::jumpstats::create,
		routes::servers::refresh_token,
	),

	components(
		schemas(
			crate::Error,
			cs2kz::SteamID,
			cs2kz::Mode,
			cs2kz::Style,
			cs2kz::Jumpstat,
			routes::players::CreatePlayer,
			routes::players::UpdatePlayer,
			routes::records::RecordRequest,
			routes::jumpstats::JumpstatRequest,
		),

		responses(
			responses::BadRequest,
			responses::Unauthorized,
			responses::Database,
		),
	),
)]
pub struct API;

impl API {
	/// Creates an [`axum::Router`] which can be served as a tower service.
	pub fn router(state: AppState) -> Router {
		let state: &'static AppState = Box::leak(Box::new(state));

		let cs_server_auth =
			axum::middleware::from_fn_with_state(state, middleware::server_auth::verify_server);

		let cs_server_router = Router::new()
			.route("/players", routing::post(routes::players::create))
			.route("/players", routing::put(routes::players::update))
			.route("/records", routing::post(routes::records::create))
			.route("/jumpstats", routing::post(routes::jumpstats::create))
			.route("/servers/refresh_token", routing::post(routes::servers::refresh_token))
			.layer(cs_server_auth)
			.with_state(state);

		let public_api_router = Router::new().route("/health", routing::get(routes::health));
		let api_router = cs_server_router.merge(public_api_router);
		let swagger_ui = Self::swagger_ui();
		let cors = CorsLayer::new()
			.allow_methods([Method::GET])
			.allow_origin(cors::Any);

		Router::new()
			.nest("/api/v0", api_router)
			.layer(cors)
			.merge(swagger_ui)
	}

	/// Creates an iterator over all of the API's routes.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_keys()
	}

	/// Creates a tower service layer for serving an HTML page with SwaggerUI.
	pub fn swagger_ui() -> SwaggerUi {
		SwaggerUi::new("/api/docs/swagger-ui").url("/api/docs/openapi.json", Self::openapi())
	}
}
