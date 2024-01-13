use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use axum::Router;

use crate::State;

pub mod openapi;
pub mod permissions;
pub mod servers;
pub mod steam;

pub mod jwt;
pub use jwt::JWT;

pub mod session;
pub use session::Session;

pub fn router(state: Arc<State>) -> Router {
	Router::new()
		.nest("/steam", steam::router(Arc::clone(&state)))
		.nest("/servers", servers::router(state))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Subdomain {
	Dashboard,
	Forum,
	Docs,
}

impl FromStr for Subdomain {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"dashboard" => Ok(Self::Dashboard),
			"forum" => Ok(Self::Forum),
			"docs" => Ok(Self::Docs),
			_ => Err(()),
		}
	}
}

impl fmt::Display for Subdomain {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Subdomain::Dashboard => "dashboard",
			Subdomain::Forum => "forum",
			Subdomain::Docs => "docs",
		})
	}
}
