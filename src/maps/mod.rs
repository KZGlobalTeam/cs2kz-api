use axum::http::Method;
use axum::routing::{get, patch, put};
use axum::Router;

use crate::middleware::auth;
use crate::{cors, State};

mod queries;

pub mod models;
pub use models::{CourseUpdate, CreatedMap, FilterUpdate, KZMap, MapUpdate, NewMap};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = auth::layer!(Maps with state);

	let root = Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::permissive(Method::GET))
		.route("/", put(routes::create).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PUT))
		.with_state(state);

	let ident = Router::new()
		.route("/:map", get(routes::get_single))
		.route_layer(cors::permissive(Method::GET))
		.route("/:map", patch(routes::update).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PATCH))
		.with_state(state);

	root.merge(ident)
}

/// Helper enum for inserting mappers into the database.
#[derive(Debug)]
enum MappersTable {
	/// The `Mappers` table should be used.
	Map(u16),

	/// The `CourseMappers` table should be used.
	Course(u32),
}
