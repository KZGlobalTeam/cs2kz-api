use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::State;

pub mod models;
pub use models::AuthRequest;

pub mod open_id;
pub use open_id::LoginForm;

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	Router::new()
		.route("/login", get(routes::login))
		.route("/logout", get(routes::logout))
		.route("/callback", get(routes::callback))
		.with_state(state)
}
