use std::sync::Arc;

use axum::routing::{delete, get, put};
use axum::Router;

use super::Permissions;
use crate::{middleware, State};

pub mod models;
pub use models::{Admin, NewAdmin};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			Arc::clone(&state),
			middleware::auth::web::layer::<{ Permissions::MANAGE_ADMINS.0 }>,
		)
	};

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", put(routes::update).layer(auth()))
		.route("/:steam_id", get(routes::get_single))
		.route("/:steam_id", delete(routes::delete).layer(auth()))
		.with_state(state)
}
