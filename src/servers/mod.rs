use std::sync::Arc;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::auth::permissions::Permissions;
use crate::{middleware, State};

mod queries;

pub mod models;
pub use models::{CreatedServer, NewServer, Server, ServerUpdate};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let approve_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::SERVERS_APPROVE.0 }>,
	);

	let update_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::SERVERS_EDIT.0 }>,
	);

	let deglobal_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::SERVERS_DEGLOBAL.0 }>,
	);

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create).layer(approve_layer))
		.route("/:server", get(routes::get_single))
		.route("/:server", patch(routes::update).layer(update_layer.clone()))
		.route("/:server/key", put(routes::replace_key).layer(update_layer))
		.route("/:server/key", delete(routes::delete_key).layer(deglobal_layer))
		.with_state(state)
}
