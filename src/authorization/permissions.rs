//! This module contains the [`Permissions`] type, which abstracts over this idea. It has various
//! constants defined for the existing roles, and can be serialized / deserialized by serde, as
//! well as be inserted into the database.

crate::bitflags! {
	/// Bitfield for holding permission information.
	///
	/// Every permission is represented as a specific bit in a 32-bit integer.
	/// If the bit is 1, it means the user has this permission.
	pub Permissions as u32 {
		BANS = { 1 << 0, "bans" };
		SERVERS = { 1 << 8, "servers" };
		MAPS = { 1 << 16, "maps" };
		ADMIN = { 1 << 31, "admin" };
	}

	iter: PermissionsIter
}
