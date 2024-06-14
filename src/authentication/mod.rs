//! Everything related to authentication.
//!
//! This module contains types, traits, and HTTP handlers related to authentication.
//! This includes JWT, sessions, and opaque API keys.

use axum::http::Method;
use axum::{routing, Router};

use crate::middleware::cors;
use crate::State;

mod jwt;
pub use jwt::Jwt;

mod server;
pub use server::Server;

pub mod session;
pub use session::Session;

pub mod api_key;
pub use api_key::ApiKey;

mod user;
pub use user::User;

pub mod steam;

pub mod handlers;

/// Returns a [Router] with all the `/auth` handlers.
pub fn router(state: State) -> Router {
	let logout = Router::new()
		.route("/logout", routing::get(handlers::logout))
		.route_layer(cors::dashboard([Method::GET]))
		.with_state(state.clone());

	Router::new()
		.route("/login", routing::get(handlers::login))
		.route("/callback", routing::get(handlers::callback))
		.route_layer(cors::permissive())
		.with_state(state.clone())
		.merge(logout)
}
