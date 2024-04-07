//! Everything related to authentication.

use axum::http::Method;
use axum::routing::get;
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod jwt;
pub use jwt::Jwt;

pub mod role_flags;
pub use role_flags::RoleFlags;

mod key;
pub use key::Key;

pub mod session;
pub use session::Session;

mod authorization;
pub use authorization::{AuthorizeSession, Either, HasRoles, None, ServerOwner};

pub mod models;
pub use models::{Server, SteamLoginForm, SteamLoginResponse, SteamUser, User};

pub mod handlers;

/// Returns a router with routes for `/auth`.
pub fn router(state: &'static State) -> Router {
	let logout = Router::new()
		.route("/logout", get(handlers::logout))
		.route_layer(cors::dashboard([Method::GET]))
		.with_state(state);

	Router::new()
		.route("/login", get(handlers::login))
		.route("/callback", get(handlers::callback))
		.route_layer(cors::permissive())
		.with_state(state)
		.merge(logout)
}
