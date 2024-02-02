use axum::http::Method;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::auth::Role;
use crate::{cors, middleware, State};

mod queries;

pub mod models;
pub use models::{CreatedServer, NewServer, Server, ServerUpdate};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			state,
			middleware::auth::web::layer::<{ Role::Servers as u32 }>,
		)
	};

	let root = Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create).route_layer(auth()))
		.route_layer(cors::dashboard([Method::GET, Method::POST]))
		.with_state(state);

	let ident = Router::new()
		.route("/key", post(routes::create_jwt))
		.route("/:server", get(routes::get_single))
		.route("/:server", patch(routes::update).route_layer(auth()))
		.route_layer(cors::dashboard([Method::GET, Method::POST, Method::PATCH]))
		.with_state(state);

	let server_key = Router::new()
		.route("/:server/key", put(routes::replace_key))
		.route("/:server/key", delete(routes::delete_key).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PUT, Method::DELETE]))
		.with_state(state);

	root.merge(ident).merge(server_key)
}
