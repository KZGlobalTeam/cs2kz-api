use axum::http::Method;
use axum::routing::{get, put};
use axum::Router;

use crate::middleware::auth;
use crate::{cors, State};

pub mod models;
pub use models::Admin;

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = auth::layer!(Admin with state);

	let root = Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::permissive(Method::GET))
		.with_state(state);

	let ident = Router::new()
		.route("/:steam_id", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.route("/:steam_id", put(routes::update).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PUT))
		.with_state(state);

	root.merge(ident)
}
