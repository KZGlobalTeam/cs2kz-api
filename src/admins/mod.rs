//! Everything related to "admins".
//!
//! "Admins" in this case means users with some special [permissions].
//! These permissions can be managed using the endpoints in this module.
//!
//! [permissions]: crate::authorization::Permissions

use axum::http::Method;
use axum::routing::{get, put};
use axum::Router;

use crate::authorization::Permissions;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{authorization, State};

mod models;
pub use models::{Admin, AdminUpdate};

pub mod handlers;

/// Returns a router with routes for `/admins`.
pub fn router(state: State) -> Router {
	let auth = session_auth!(
		authorization::HasPermissions<{ Permissions::ADMIN.value() }>,
		state.clone(),
	);

	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let by_id = Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.route("/:id", put(handlers::by_id::put).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state.clone());

	root.merge(by_id)
}
