//! Everything related to managing CS2KZ plugin versions.

use axum::routing::{get, post};
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CreatedPluginVersion, NewPluginVersion, PluginVersion, PluginVersionID};

pub mod handlers;

/// Returns a router with routes for `/plugin`.
pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/versions", get(handlers::versions::get))
		.route_layer(cors::permissive())
		.route("/versions", post(handlers::versions::post))
		.with_state(state)
}
