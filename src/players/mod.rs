//! Everything related to KZ players.

use axum::{routing, Router};

use crate::middleware::cors;
use crate::State;

mod models;
pub use models::{
	CourseSession, CourseSessions, FullPlayer, NewPlayer, Player, PlayerUpdate, Session,
};

mod queries;
pub mod handlers;

/// Returns an [`axum::Router`] for the `/players` routes.
pub fn router(state: State) -> Router {
	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::post(handlers::root::post))
		.with_state(state.clone());

	let by_identifier = Router::new()
		.route("/:player", routing::get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:player", routing::patch(handlers::by_identifier::patch))
		.with_state(state.clone());

	let steam = Router::new()
		.route("/:player/steam", routing::get(handlers::steam::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let preferences = Router::new()
		.route(
			"/:player/preferences",
			routing::get(handlers::preferences::get),
		)
		.route_layer(cors::permissive())
		.with_state(state.clone());

	root.merge(by_identifier).merge(steam).merge(preferences)
}
