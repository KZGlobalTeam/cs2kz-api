use std::sync::Arc;

use axum::routing::get;
use axum::Router;

pub mod models;
pub use models::{Auth, LoginForm};

pub mod routes;

pub fn router(state: Arc<crate::State>) -> Router {
	Router::new()
		.route("/callback", get(routes::callback))
		.with_state(state)
}
