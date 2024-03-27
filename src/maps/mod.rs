//! Everything related to maps.

use axum::http::Method;
use axum::routing::{get, patch, put};
use axum::Router;

use crate::middleware::cors;
use crate::State;

pub mod models;
pub use models::{CourseInfo, CreatedMap, FullMap, MapInfo, MapUpdate, NewMap};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/maps`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", put(handlers::root::put))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:map", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:map", patch(handlers::by_identifier::patch))
		.route_layer(cors::dashboard([Method::PATCH]))
		.with_state(state);

	root.merge(by_identifier)
}
