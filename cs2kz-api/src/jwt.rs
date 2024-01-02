//! This module holds various types that are encoded into JWTs.

use serde::{Deserialize, Serialize};

/// Information about a server.
///
/// This struct will be turned into a JWT and given to servers so they can authenticate any
/// requests they make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerClaims {
	/// The server's ID.
	pub id: u16,

	/// The CS2KZ version the server is running on.
	pub plugin_version_id: u16,

	/// Timestamp of when this token expires.
	#[serde(rename = "exp")]
	pub expires_at: u64,
}

impl ServerClaims {
	/// Constructs a new token. It will expire after 30 minutes.
	pub fn new(id: u16, plugin_version_id: u16) -> Self {
		Self {
			id,
			plugin_version_id,
			expires_at: jwt::get_current_timestamp() + (60 * 30),
		}
	}
}
