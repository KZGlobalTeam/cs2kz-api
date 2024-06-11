//! Everything related to servers.

use axum::http::Method;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::authorization::Permissions;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{authorization, State};

mod models;
pub use models::{
	AccessKeyRequest, AccessKeyResponse, CreatedServer, NewServer, RefreshKey, Server, ServerID,
	ServerInfo, ServerUpdate,
};

mod queries;
pub mod handlers;

/// Returns a router with routes for `/servers`.
pub fn router(state: State) -> Router {
	let is_admin = session_auth!(
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
		state.clone(),
	);

	let is_admin_or_owner = session_auth!(authorization::IsServerAdminOrOwner, state.clone());

	let root = Router::new()
		.route("/", get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", post(handlers::root::post).route_layer(is_admin()))
		.route_layer(cors::dashboard([Method::POST]))
		.with_state(state.clone());

	let key = Router::new()
		.route("/key", post(handlers::key::generate_temp))
		.with_state(state.clone());

	let by_identifier = Router::new()
		.route("/:server", get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route(
			"/:server",
			patch(handlers::by_identifier::patch).route_layer(is_admin_or_owner()),
		)
		.route_layer(cors::dashboard([Method::PATCH]))
		.with_state(state.clone());

	let by_identifier_key = Router::new()
		.route(
			"/:server/key",
			put(handlers::key::put_perma).route_layer(is_admin_or_owner()),
		)
		.route(
			"/:server/key",
			delete(handlers::key::delete_perma).route_layer(is_admin()),
		)
		.route_layer(cors::dashboard([Method::PUT, Method::DELETE]))
		.with_state(state.clone());

	root.merge(key)
		.merge(by_identifier)
		.merge(by_identifier_key)
}
