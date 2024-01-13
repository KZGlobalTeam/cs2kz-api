use std::sync::Arc;

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::auth::permissions::Permissions;
use crate::{middleware, State};

mod queries;

pub mod models;
pub use models::{Ban, BanUpdate, CreatedBan, CreatedUnban, NewBan, NewUnban};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let ban_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::BANS_CREATE.0 }>,
	);

	let update_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::BANS_EDIT.0 }>,
	);

	let unban_layer = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Permissions::BANS_REMOVE.0 }>,
	);

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create).layer(ban_layer))
		.route("/:id", get(routes::get_single))
		.route("/:id", patch(routes::update).layer(update_layer))
		.route("/:id", delete(routes::unban).layer(unban_layer))
		.with_state(state)
}
