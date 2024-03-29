//! Everything related to players.

use axum::http::Method;
use axum::routing::{get, patch, post, put};
use axum::Router;

use crate::middleware::cors;
use crate::State;

pub mod models;
pub use models::{FullPlayer, NewPlayer, Player, PlayerUpdate};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/players`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:player", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:player", patch(handlers::by_identifier::patch))
		.with_state(state);

	let preferences = Router::new()
		.route("/:player/preferences", get(handlers::preferences::get))
		.route_layer(cors::permissive())
		.route("/:player/preferences", put(handlers::preferences::put))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state);

	root.merge(by_identifier).merge(preferences)
}
