//! Everything related to KZ admins.

use axum::http::Method;
use axum::{routing, Router};

use crate::authorization::Permissions;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{authorization, State};

mod models;
pub use models::{Admin, AdminUpdate};

pub mod handlers;

/// Returns an [`axum::Router`] for the `/admins` routes.
pub fn router(state: State) -> Router {
	let auth = session_auth!(
		authorization::HasPermissions<{ Permissions::ADMIN.value() }>,
		state.clone(),
	);

	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.with_state(state.clone());

	let by_id = Router::new()
		.route("/:id", routing::get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.route(
			"/:id",
			routing::put(handlers::by_id::put).route_layer(auth()),
		)
		.route_layer(cors::dashboard([Method::PUT]))
		.with_state(state.clone());

	root.merge(by_id)
}
