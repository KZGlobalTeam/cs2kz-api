use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::Role;

/// Response body for `/admins` endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Admin {
	/// The admin's latest known name.
	pub name: String,

	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's roles.
	pub roles: Vec<Role>,
}
