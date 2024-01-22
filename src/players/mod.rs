use std::sync::Arc;

use axum::routing::{get, post, put};
use axum::Router;

use crate::auth::Role;
use crate::{middleware, State};

pub mod models;
pub use models::{Admin, NewPlayer, Player};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let auth = axum::middleware::from_fn_with_state(
		Arc::clone(&state),
		middleware::auth::web::layer::<{ Role::Admin as u32 }>,
	);

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create))
		.route("/admins", get(routes::get_admins))
		.route("/:player", get(routes::get_single))
		.route("/:player/roles", get(routes::get_roles))
		.route("/:player/roles", put(routes::update_roles).layer(auth))
		.with_state(state)
}
