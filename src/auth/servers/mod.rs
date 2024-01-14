use std::sync::Arc;

use axum::routing::post;
use axum::Router;

use crate::State;

pub mod models;
pub use models::{AuthenticatedServer, ServerAccessToken};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	Router::new()
		.route("/refresh", post(routes::refresh_key))
		.with_state(state)
}
