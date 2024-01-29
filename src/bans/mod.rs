use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::auth::Role;
use crate::{middleware, State};

mod queries;

pub mod models;
pub use models::{Ban, BanUpdate, CreatedBan, CreatedUnban, NewBan, NewUnban};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			state,
			middleware::auth::web::layer::<{ Role::Bans as u32 }>,
		)
	};

	Router::new()
		.route("/", get(routes::get_many))
		.route("/", post(routes::create).layer(auth()))
		.route("/:id", get(routes::get_single))
		.route("/:id", patch(routes::update).layer(auth()))
		.route("/:id", delete(routes::unban).layer(auth()))
		.with_state(state)
}
