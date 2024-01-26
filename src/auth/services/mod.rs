use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::State;

pub mod models;
pub use models::{NewService, Service};

use super::Role;

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let auth = axum::middleware::from_extractor_with_state::<Service<{ Role::Admin as u32 }>, _>(
		Arc::clone(&state),
	);

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create))
		.route("/:id", get(routes::get_single))
		.route("/:id", delete(routes::delete))
		.route("/:id/key", put(routes::update_key))
		.layer(auth)
		.with_state(state)
}
