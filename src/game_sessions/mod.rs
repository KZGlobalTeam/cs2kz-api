//! Everything related to KZ game sessions.

use axum::routing::get;
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CourseSessionID, GameSession, GameSessionID, TimeSpent};

pub mod handlers;

/// Returns an [`axum::Router`] for the `/sessions` routes.
pub fn router(state: State) -> Router {
	Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.with_state(state.clone())
}
