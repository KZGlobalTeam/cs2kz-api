use axum::http::Method;
use axum::routing::get;
use axum::Router;

use crate::{cors, State};

mod roles;
pub use roles::{Role, RoleFlags};

pub mod steam;

mod users;
pub use users::User;

pub mod sessions;
pub use sessions::Session;

mod jwt;
pub use jwt::Jwt;

pub mod servers;
pub use servers::Server;

pub mod routes;
pub mod openapi;

pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/login", get(routes::login))
		.route_layer(cors::permissive(Method::GET))
		.route("/logout", get(routes::logout))
		.route_layer(cors::dashboard(Method::GET))
		.with_state(state)
		.nest("/steam", steam::router(state))
}
