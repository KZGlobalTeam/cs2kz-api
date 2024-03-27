//! Everything related to "admins".
//!
//! "Admins" in this case means users with some special permissions.
//! These permissions are assigned in the form of [roles], which can be managed using the
//! endpoints in this module.
//!
//! [roles]: crate::auth::role_flags

use axum::http::Method;
use axum::routing::{get, put};
use axum::Router;

use crate::middleware::cors;
use crate::State;

pub mod models;
pub use models::{Admin, AdminUpdate};

pub mod handlers;

/// Returns a router with routes for `/admins`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.with_state(state);

	let by_id = Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.route("/:id", put(handlers::by_id::put))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state);

	root.merge(by_id)
}
