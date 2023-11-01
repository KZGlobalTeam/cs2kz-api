use {
	crate::state::AppState,
	axum::{extract::State as StateExtractor, routing, Router},
	utoipa::OpenApi,
	utoipa_swagger_ui::SwaggerUi,
};

pub mod error;
pub use error::{Error, Result};

pub mod util;
pub mod database;

pub mod logging;
pub mod routes;
pub mod state;
pub mod res;

/// Type alias for easy use in function signatures.
///
/// You can read more about axum's extractors
/// [here](https://docs.rs/axum/0.6.20/axum/index.html#extractors).
///
/// Usually you would write a handler function like this:
///
/// ```rust
/// use axum::extract::State;
/// use crate::State as AppState;
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

pub type Response<T> = Result<axum::Json<T>>;

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
		routes::players::get::root,
		routes::players::get::ident,
		routes::players::post::root,
		routes::players::put::steam_id,
	),

	components(
		schemas(
			crate::Error,
			cs2kz::SteamID,
			cs2kz::PlayerIdentifier,
			cs2kz::Mode,
			cs2kz::Style,
			cs2kz::Jumpstat,

			res::player::Player,

			routes::players::post::NewPlayer,
			routes::players::put::PlayerUpdate,
		),

		responses(
			res::BadRequest,
		),
	),
)]
pub struct API;

impl API {
	/// Creates an [`axum::Router`] which can be served as a tower service.
	pub fn router(state: AppState) -> Router {
		let state: &'static AppState = Box::leak(Box::new(state));

		let public_api_router = Router::new()
			.route("/health", routing::get(routes::health))
			.route("/players", routing::get(routes::players::get::root))
			.route("/players/:ident", routing::get(routes::players::get::ident))
			.with_state(state);

		let game_server_router = Router::new()
			.route("/players", routing::post(routes::players::post::root))
			.route("/players/:ident", routing::put(routes::players::put::steam_id))
			.with_state(state);

		let api_router = game_server_router.merge(public_api_router);

		let swagger_ui = Self::swagger_ui();

		Router::new()
			.nest("/api/v0", api_router)
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
