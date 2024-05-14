//! Everything related to maps.

use axum::http::Method;
use axum::routing::{get, patch, put};
use axum::Router;

use crate::auth::RoleFlags;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{auth, State};

mod models;

#[doc(inline)]
pub use models::{
	Course, CourseID, CourseInfo, CourseUpdate, CreatedMap, Filter, FilterID, FilterUpdate,
	FullMap, MapID, MapInfo, MapUpdate, NewCourse, NewFilter, NewMap,
};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/maps`.
pub fn router(state: &'static State) -> Router {
	let auth = session_auth!(auth::HasRoles<{ RoleFlags::MAPS.value() }>, state);

	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", put(handlers::root::put).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:map", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route(
			"/:map",
			patch(handlers::by_identifier::patch).route_layer(auth()),
		)
		.route_layer(cors::dashboard([Method::PATCH]))
		.with_state(state);

	root.merge(by_identifier)
}
