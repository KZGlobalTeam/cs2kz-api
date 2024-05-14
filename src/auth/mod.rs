//! Everything related to authentication.

use axum::http::Method;
use axum::routing::get;
use axum::Router;

use crate::middleware::cors;
use crate::State;

mod jwt;

#[doc(inline)]
pub use jwt::Jwt;

mod role_flags;

#[doc(inline)]
pub use role_flags::RoleFlags;

mod key;

#[doc(inline)]
pub use key::Key;

mod session;

#[doc(inline)]
pub use session::Session;

mod authorization;

#[doc(inline)]
pub use authorization::{AdminOrServerOwner, AuthorizeSession, HasRoles, None};

mod models;

#[doc(inline)]
pub use models::{Server, SteamLoginForm, SteamLoginResponse, User};

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
