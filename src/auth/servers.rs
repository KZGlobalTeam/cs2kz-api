use chrono::Duration;
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Jwt;
use crate::state::{jwt, Result};

/// An authenticated server.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Server {
	/// The server's unique ID.
	pub id: u16,

	/// The ID of the cs2kz version the server is currently running.
	pub plugin_version_id: u16,
}

impl Server {
	pub const fn new(id: u16, plugin_version_id: u16) -> Self {
		Self { id, plugin_version_id }
	}

	/// Generates an access token (JWT) for this server.
	pub fn into_jwt(self, state: &jwt::State) -> Result<AccessToken> {
		let expires_after = Duration::minutes(30);
		let jwt = Jwt::new(self, expires_after);
		let jwt = state.encode(&jwt)?;

		Ok(AccessToken(jwt))
	}
}

/// A semi-permanent authentication token used by servers to generate [`AccessToken`]s.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefreshToken {
	/// The server's API key.
	pub key: u32,

	/// Semver cs2kz plugin version the server is currently running.
	#[schema(value_type = String)]
	pub plugin_version: Version,
}

/// JWT for server authentication.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct AccessToken(pub String);
