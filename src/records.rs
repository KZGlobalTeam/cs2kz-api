use axum::http::Method;
use axum::routing::{get, post};
use axum::Router;

use crate::{cors, State};

mod queries;

pub mod models;
pub use models::{CreatedRecord, NewRecord, Record};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::permissive(Method::GET))
		// TODO: AC middleware
		.route("/", post(routes::create))
		.with_state(state);

	let ident = Router::new()
		.route("/:record_id", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.with_state(state);

	root.merge(ident)
}
