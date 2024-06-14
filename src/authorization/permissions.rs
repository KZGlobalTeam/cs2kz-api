//! User permissions.

crate::bitflags! {
	/// Bitflags containing all the possible permissions a user can have.
	pub Permissions as u32 {
		BANS = { 1 << 0, "bans" };
		SERVERS = { 1 << 8, "servers" };
		MAPS = { 1 << 16, "maps" };
		ADMIN = { 1 << 31, "admin" };
	}

	iter: PermissionsIter
}
