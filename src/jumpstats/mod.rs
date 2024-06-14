//! Everything related to jumpstats.

use axum::{routing, Router};

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CreatedJumpstat, Jumpstat, JumpstatID, NewJumpstat};

mod queries;
pub mod handlers;

/// Returns an [`axum::Router`] for the `/jumpstats` routes.
pub fn router(state: State) -> Router {
	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::post(handlers::root::post))
		.with_state(state.clone());

	let by_id = Router::new()
		.route("/:id", routing::get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let replay = Router::new()
		.route("/:id/replay", routing::get(handlers::replays::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	root.merge(by_id).merge(replay)
}
