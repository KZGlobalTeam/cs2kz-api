use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::auth::Permission;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Admin {
	pub steam_id: SteamID,
	pub name: String,
	pub permissions: Vec<Permission>,
}

#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct NewAdmin {
	pub steam_id: SteamID,
	pub permissions: Vec<Permission>,
}
