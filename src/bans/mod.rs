//! Everything related to bans.

use axum::http::Method;
use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::auth::RoleFlags;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{auth, State};

pub mod models;
pub use models::{Ban, BanReason, BanUpdate, CreatedBan, CreatedUnban, NewBan, NewUnban};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/bans`.
pub fn router(state: &'static State) -> Router {
	let auth = session_auth!(auth::HasRoles<{ RoleFlags::BANS.as_u32() }>, state);

	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post))
		.route_layer(cors::dashboard([Method::POST]))
		.with_state(state);

	let by_id = Router::new()
		.route("/:id", get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.route("/:id", patch(handlers::by_id::patch).route_layer(auth()))
		.route("/:id", delete(handlers::by_id::delete).route_layer(auth()))
		.route_layer(cors::dashboard([Method::PATCH, Method::DELETE]))
		.with_state(state);

	root.merge(by_id)
}
