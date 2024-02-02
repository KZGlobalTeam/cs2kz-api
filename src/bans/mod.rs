use axum::http::Method;
use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::auth::Role;
use crate::{cors, middleware, State};

mod queries;

pub mod models;
pub use models::{Ban, BanUpdate, CreatedBan, CreatedUnban, NewBan, NewUnban};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			state,
			middleware::auth::web::layer::<{ Role::Bans as u32 }>,
		)
	};

	let root = Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::permissive(Method::GET))
		.route("/", post(routes::create).route_layer(auth()))
		.route_layer(cors::dashboard(Method::POST))
		.with_state(state);

	let id = Router::new()
		.route("/:id", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.route("/:id", patch(routes::update).route_layer(auth()))
		.route("/:id", delete(routes::unban).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PATCH, Method::DELETE]))
		.with_state(state);

	root.merge(id)
}
