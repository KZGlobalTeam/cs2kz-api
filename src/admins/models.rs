//! Types used for describing KZ admins.

use cs2kz::SteamID;
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::RoleFlags;

/// A player with special privileges.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Admin {
	/// The admin's name.
	pub name: String,

	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's roles.
	#[debug("{roles:?} ({roles})")]
	#[schema(value_type = Vec<String>, example = json!(["bans", "servers"]))]
	pub roles: RoleFlags,
}

/// Request body for updating admins.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdate {
	/// New roles for the admin.
	#[debug("{roles}")]
	#[schema(value_type = Vec<String>, example = json!(["bans", "servers"]))]
	pub roles: RoleFlags,
}
