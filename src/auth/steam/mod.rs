use axum::routing::get;
use axum::Router;

use crate::{cors, State};

pub mod models;
pub use models::{Auth, LoginForm};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/callback", get(routes::callback))
		.route_layer(cors::get())
		.with_state(state)
}
