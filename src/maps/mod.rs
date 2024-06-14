//! Everything related to KZ maps.

use axum::http::Method;
use axum::{routing, Router};

use crate::authorization::Permissions;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{authorization, State};

mod models;
pub use models::{
	Course, CourseID, CourseInfo, CourseUpdate, CreatedMap, Filter, FilterID, FilterUpdate,
	FullMap, MapID, MapInfo, MapUpdate, NewCourse, NewFilter, NewMap,
};

mod queries;
pub mod handlers;

/// Returns an [`axum::Router`] for the `/maps` routes.
pub fn router(state: State) -> Router {
	let auth = session_auth!(
		authorization::HasPermissions<{ Permissions::MAPS.value() }>,
		state.clone(),
	);

	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::put(handlers::root::put).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state.clone());

	let by_identifier = Router::new()
		.route("/:map", routing::get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route(
			"/:map",
			routing::patch(handlers::by_identifier::patch).route_layer(auth()),
		)
		.route_layer(cors::dashboard([Method::PATCH]))
		.with_state(state.clone());

	root.merge(by_identifier)
}
