//! Everything related to records.

use axum::routing::{get, post};
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{BhopStats, CreatedRecord, NewRecord, Record, RecordID};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/records`.
pub fn router(state: State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.with_state(state.clone());

	let top = Router::new()
		.route("/top", get(handlers::top::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let by_id = Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let replay = Router::new()
		.route("/:id/replay", get(handlers::replays::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	root.merge(top).merge(by_id).merge(replay)
}
