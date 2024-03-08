use axum::http::Method;
use axum::routing::get;
use axum::Router;

use crate::{cors, State};

mod queries;

pub mod models;
pub use models::Session;

pub mod routes;

pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/", get(routes::get_many))
		.route("/:session_id", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.with_state(state)
}
