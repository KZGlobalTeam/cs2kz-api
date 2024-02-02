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

	Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::get())
		.route("/", post(routes::create).route_layer(auth()))
		.route_layer(cors::dashboard(Method::POST))
		.route("/key", post(routes::create_jwt))
		.route_layer(cors::post())
		.route("/:server", get(routes::get_single))
		.route_layer(cors::get())
		.route("/:server", patch(routes::update).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PATCH))
		.route("/:server/key", put(routes::replace_key))
		.route_layer(cors::dashboard(Method::PUT))
		.route("/:server/key", delete(routes::delete_key).route_layer(auth()))
		.route_layer(cors::dashboard(Method::DELETE))
		.with_state(state)
}
