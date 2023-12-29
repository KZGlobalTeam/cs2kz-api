//! This module (and its sub-modules) hold all HTTP handler functions.

use axum::routing::get;
use axum::Router;
use tower_http::cors::{self, AllowMethods, CorsLayer};

use crate::state::AppState;

pub mod players;
pub mod maps;
pub mod servers;
pub mod jumpstats;
pub mod records;
pub mod bans;
pub mod auth;

/// Creates the main API router by composing other routers defined in sub-modules.
pub fn router(state: &'static AppState) -> Router {
	let log_request = axum::middleware::from_fn(crate::middleware::log_request);

	// FIXME(AlphaKeks)
	let cors = CorsLayer::new()
		.allow_methods(AllowMethods::any())
		.allow_origin(cors::Any);

	Router::new()
		.route("/", get(status))
		.nest("/players", players::router(state))
		.nest("/maps", maps::router(state))
		.nest("/servers", servers::router(state))
		.nest("/jumpstats", jumpstats::router(state))
		.nest("/records", records::router(state))
		.nest("/bans", bans::router(state))
		.nest("/auth", auth::router(state))
		.layer(cors)
		.layer(log_request)
}

/// The default response from the API.
///
/// If this endpoint does not respond, something is wrong.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Status",
	path = "/",
	responses(
		(status = 200, description = "The API is up.", body = String, example = json!("(͡ ͡° ͜ つ ͡͡°)")),
	),
)]
async fn status() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
