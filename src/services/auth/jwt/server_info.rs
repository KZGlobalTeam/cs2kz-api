//! CS2 server information that can be encoded in JWTs.

use serde::{Deserialize, Serialize};

use crate::services::plugin::PluginVersionID;
use crate::services::servers::ServerID;

/// JWT payload for an authenticated CS2 servers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ServerInfo
{
	/// The server's ID.
	id: ServerID,

	/// The ID of the CS2KZ version the server is currently running.
	plugin_version_id: PluginVersionID,
}

impl ServerInfo
{
	/// Creates a new [`ServerInfo`].
	pub fn new(id: ServerID, plugin_version_id: PluginVersionID) -> Self
	{
		Self { id, plugin_version_id }
	}

	/// Returns the server's ID.
	pub fn id(&self) -> ServerID
	{
		self.id
	}

	/// Returns the ID of the CS2KZ version the server is currently running.
	pub fn plugin_version_id(&self) -> PluginVersionID
	{
		self.plugin_version_id
	}
}
