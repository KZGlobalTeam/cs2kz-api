use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The temporary access token used by CS2 servers for authenticating requests.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthResponse {
	pub access_token: String,
}

/// JWT payload for CS2 servers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AuthenticatedServer {
	/// The server's ID.
	pub id: u16,

	/// The ID of the cs2kz version which the server is currently running on.
	pub plugin_version_id: u16,
}

impl AuthenticatedServer {
	pub fn new(id: u16, plugin_version_id: u16) -> Self {
		Self { id, plugin_version_id }
	}
}
