//! Everything related to servers.

use axum::http::Method;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::middleware::cors;
use crate::State;

pub mod models;
pub use models::{
	CreatedServer, NewServer, RefreshKey, RefreshKeyRequest, RefreshKeyResponse, Server,
	ServerInfo, ServerUpdate,
};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/servers`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.route_layer(cors::dashboard([Method::POST]))
		.with_state(state);

	let key = Router::new()
		.route("/key", post(handlers::key::generate_temp))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:server", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:server", patch(handlers::by_identifier::patch))
		.route_layer(cors::dashboard([Method::PATCH]))
		.with_state(state);

	let by_identifier_key = Router::new()
		.route("/:server/key", put(handlers::key::put_perma))
		.route("/:server/key", delete(handlers::key::delete_perma))
		.route_layer(cors::dashboard([Method::PUT, Method::DELETE]))
		.with_state(state);

	root.merge(key)
		.merge(by_identifier)
		.merge(by_identifier_key)
}
