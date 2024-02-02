use axum::http::Method;
use axum::routing::{get, put};
use axum::Router;

use crate::auth::Role;
use crate::{cors, middleware, State};

pub mod models;
pub use models::Admin;

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = axum::middleware::from_fn_with_state(
		state,
		middleware::auth::web::layer::<{ Role::Admin as u32 }>,
	);

	let root = Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::permissive(Method::GET))
		.with_state(state);

	let ident = Router::new()
		.route("/:steam_id", get(routes::get_single))
		.route("/:steam_id", put(routes::update).route_layer(auth))
		.route_layer(cors::dashboard([Method::GET, Method::PUT]))
		.with_state(state);

	root.merge(ident)
}
