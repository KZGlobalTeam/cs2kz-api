use cs2kz::SteamID;

use super::RoleFlags;

#[derive(Debug, Clone, Copy)]
pub struct User {
	/// The [SteamID] of the user.
	pub steam_id: SteamID,

	/// The roles of the user for this session.
	pub role_flags: RoleFlags,
}

impl User {
	pub const fn new(steam_id: SteamID, role_flags: RoleFlags) -> Self {
		Self { steam_id, role_flags }
	}
}
