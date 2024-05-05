//! Helper type for dealing with user roles.

crate::bitflags::bitflags! {
	/// Bitfield for holding role information.
	///
	/// Every role is represented as a specific bit in a 32-bit integer.
	/// If the bit is 1, it means the user has this role.
	///
	/// This module contains the [`RoleFlags`] type, which abstracts over this idea.
	/// It has various constants defined for the existing roles, and can be serialized / deserialized
	/// by serde, as well as be inserted into the database.
	pub RoleFlags as u32 {
		BANS = { 1 << 0, "bans" };
		SERVERS = { 1 << 8, "servers" };
		MAPS = { 1 << 16, "maps" };
		ADMIN = { 1 << 31, "admin" };
	}

	iter: RoleFlagsIter
}
