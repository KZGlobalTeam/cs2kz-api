//! Everything related to jumpstats.

use axum::routing::{get, post};
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CreatedJumpstat, Jumpstat, JumpstatID, NewJumpstat};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/jumpstats`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.with_state(state);

	let by_id = Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.with_state(state);

	let replay = Router::new()
		.route("/:id/replay", get(handlers::replays::get))
		.route_layer(cors::permissive())
		.with_state(state);

	root.merge(by_id).merge(replay)
}
