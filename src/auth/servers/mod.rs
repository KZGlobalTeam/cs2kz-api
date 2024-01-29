use axum::routing::put;
use axum::Router;

use super::Jwt;

pub mod models;
pub use models::{AccessToken, RefreshToken, Server};

pub mod routes;

pub fn router(state: &'static crate::State) -> Router {
	Router::new()
		.route("/refresh_key", put(routes::refresh_key))
		.with_state(state)
}
