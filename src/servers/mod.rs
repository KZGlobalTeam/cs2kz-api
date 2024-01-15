use std::sync::Arc;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::auth::Role;
use crate::{middleware, State};

mod queries;

pub mod models;
pub use models::{CreatedServer, NewServer, Server, ServerUpdate};

pub mod routes;

pub fn router(state: Arc<State>) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			Arc::clone(&state),
			middleware::auth::web::layer::<{ Role::Servers as u32 }>,
		)
	};

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create).layer(auth()))
		.route("/:server", get(routes::get_single))
		.route("/:server", patch(routes::update).layer(auth()))
		.route("/:server/key", put(routes::replace_key).layer(auth()))
		.route("/:server/key", delete(routes::delete_key).layer(auth()))
		.with_state(state)
}
