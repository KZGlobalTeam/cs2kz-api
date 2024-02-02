use axum::http::Method;
use axum::routing::{get, post};
use axum::Router;

use crate::{cors, State};

pub mod models;
pub use models::{NewPlayer, Player};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create))
		.route_layer(cors::permissive([Method::GET, Method::POST]))
		.with_state(state);

	let ident = Router::new()
		.route("/:player", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.with_state(state);

	root.merge(ident)
}
