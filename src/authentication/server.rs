//! Authenticated CS2 servers.
//!
//! These are community servers approved by admins that authenticate via [JWTs].
//!
//! [JWTs]: super::jwt

use derive_more::Into;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::plugin::PluginVersionID;
use crate::servers::ServerID;

/// An authenticated CS2 server.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct Server {
	/// The server's ID.
	id: ServerID,

	/// The ID of the cs2kz version the server is currently running.
	plugin_version_id: PluginVersionID,
}

impl Server {
	/// Creates a new [`Server`].
	pub const fn new(id: ServerID, plugin_version_id: PluginVersionID) -> Self {
		Self {
			id,
			plugin_version_id,
		}
	}

	/// Returns this server's ID.
	pub const fn id(&self) -> ServerID {
		self.id
	}

	/// Returns the ID of the cs2kz version this server is currently running.
	pub const fn plugin_version_id(&self) -> PluginVersionID {
		self.plugin_version_id
	}
}
