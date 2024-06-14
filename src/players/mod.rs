//! Everything related to KZ players.

use axum::routing::{get, patch, post};
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{CourseSession, FullPlayer, NewPlayer, Player, PlayerUpdate, Session};

mod queries;
pub mod handlers;

/// Returns an [`axum::Router`] for the `/players` routes.
pub fn router(state: State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.with_state(state.clone());

	let by_identifier = Router::new()
		.route("/:player", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:player", patch(handlers::by_identifier::patch))
		.with_state(state.clone());

	let steam = Router::new()
		.route("/:player/steam", get(handlers::steam::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let preferences = Router::new()
		.route("/:player/preferences", get(handlers::preferences::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	root.merge(by_identifier).merge(steam).merge(preferences)
}
