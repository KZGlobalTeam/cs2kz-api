//! Everything related to authentication.

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

mod session;
pub use session::{Either, HasRoles, IsServerOwner, Session};

pub mod models;
pub use models::{Server, SteamLoginForm, SteamLoginResponse, SteamUser, User};

pub mod handlers;

/// Returns a router with routes for `/auth`.
pub fn router(state: &'static State) -> Router {
	Router::new()
		.route("/login", get(handlers::login))
		.route("/logout", get(handlers::logout))
		.route("/callback", get(handlers::callback))
		.route_layer(cors::permissive())
		.with_state(state)
}
