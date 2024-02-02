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

	Router::new()
		.route("/", get(routes::get_many))
		.route("/:steam_id", get(routes::get_single))
		.route_layer(cors::get())
		.route("/:steam_id", put(routes::update).route_layer(auth))
		.route_layer(cors::dashboard(Method::PUT))
		.with_state(state)
}
