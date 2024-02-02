use axum::routing::{get, post};
use axum::Router;

use crate::{cors, State};

pub mod models;
pub use models::{NewPlayer, Player};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::get())
		.route("/", post(routes::create))
		.route_layer(cors::post())
		.route("/:player", get(routes::get_single))
		.route_layer(cors::get())
		.with_state(state)
}
