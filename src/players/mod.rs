use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::State;

pub mod models;
pub use models::{NewPlayer, Player};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create))
		.route("/:player", get(routes::get_single))
		.with_state(state)
}
