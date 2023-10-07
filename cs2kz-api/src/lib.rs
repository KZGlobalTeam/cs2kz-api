use {
	crate::state::AppState,
	axum::{extract::State as StateExtractor, http::Method, routing, Router},
	std::sync::Arc,
	tower_http::{cors, cors::CorsLayer},
	utoipa::OpenApi,
	utoipa_swagger_ui::SwaggerUi,
};

pub mod logging;
pub mod routes;
pub mod state;

/// Type alias for easy use in function signatures.
///
/// You can read more about axum's extractors
/// [here](https://docs.rs/axum/0.6.20/axum/index.html#extractors).
///
/// Usually you would write a handler function like this:
///
/// ```rust
/// use std::sync::Arc;
/// use crate::state::State as AppState;
/// use axum::extract::State;
///
/// async fn handler(State(state): State<Arc<AppState>>) {
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
pub type State = StateExtractor<Arc<AppState>>;

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
	),
)]
pub struct API;

impl API {
	/// Creates an [`axum::Router`] which can be served as a tower service.
	pub fn router(state: Arc<AppState>) -> Router {
		let api_router = Router::new()
			.route("/health", routing::get(routes::health))
			.with_state(state);

		let swagger_ui = Self::swagger_ui();
		let cors = CorsLayer::new()
			.allow_methods([Method::GET])
			.allow_origin(cors::Any);

		Router::new()
			.nest("/api/v1", api_router)
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
