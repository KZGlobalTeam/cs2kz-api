use std::ops::{BitAnd, BitOr};

use serde::{Deserialize, Serialize};

/// Bitfield for storing user permissions.
///
/// See associated constants for details.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Permissions(pub u64);

impl Permissions {
	/// Determines whether `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}
}

#[allow(dead_code)]
impl Permissions {
	pub const NONE: Self = Self(0);

	pub const MAPS_VIEW: Self = Self(1 << 0);
	pub const MAPS_APPROVE: Self = Self(1 << 1);
	pub const MAPS_EDIT: Self = Self(1 << 2);
	pub const MAPS_DEGLOBAL: Self = Self(1 << 3);

	pub const SERVERS_APPROVE: Self = Self(1 << 10);
	pub const SERVERS_EDIT: Self = Self(1 << 11);
	pub const SERVERS_DEGLOBAL: Self = Self(1 << 12);

	pub const BANS_CREATE: Self = Self(1 << 20);
	pub const BANS_EDIT: Self = Self(1 << 21);
	pub const BANS_REMOVE: Self = Self(1 << 22);
}

impl BitOr for Permissions {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitAnd for Permissions {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

#[cfg(test)]
mod tests {
	use super::Permissions;

	#[test]
	fn zero() {
		let zero = Permissions::default();

		assert_eq!(zero.0, 0);
		assert!(zero.contains(zero));
	}

	#[test]
	fn one() {
		let maps_edit = Permissions::MAPS_EDIT;
		let zero = Permissions::default();

		assert!(!zero.contains(maps_edit));
		assert!(maps_edit.contains(maps_edit));
		assert!(maps_edit.contains(zero));
	}

	#[test]
	fn multiple() {
		let a = Permissions::MAPS_APPROVE | Permissions::MAPS_EDIT | Permissions::MAPS_DEGLOBAL;
		let b = Permissions::MAPS_EDIT | Permissions::MAPS_DEGLOBAL;

		assert!(a.contains(b));
		assert!(!b.contains(a));
	}
}
