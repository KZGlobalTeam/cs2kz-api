//! Everything related to in-game sessions.

use axum::routing::get;
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CourseSessionID, GameSession, GameSessionID, TimeSpent};

pub mod handlers;

/// Returns a router with routes for `/sessions`.
pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.with_state(state)
}
