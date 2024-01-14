use std::sync::Arc;

use axum::routing::{get, patch, put};
use axum::Router;

use crate::auth::Permissions;
use crate::{middleware, State};

mod queries;

pub mod models;
pub use models::{CourseUpdate, CreatedMap, FilterUpdate, KZMap, MapUpdate, NewMap};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let approve_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::MAPS_APPROVE.0 }>,
	);

	let update_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::MAPS_EDIT.0 & Permissions::MAPS_DEGLOBAL.0 }>,
	);

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", put(routes::create).layer(approve_layer))
		.route("/:map", get(routes::get_single))
		.route("/:map", patch(routes::update).layer(update_layer))
		.with_state(state)
}

/// Helper enum for inserting mappers into the database.
enum MappersTable {
	Map(u16),
	Course(u32),
}
