//! Permissions for authorized requests from `*.cs2.kz` websites.

use std::ops::{BitAnd, BitOr};

use serde::{Deserialize, Serialize};

/// Bitfield helper struct for tracking user permissions.
///
/// There are a bunch of constants defined on this struct representing individual permissions.
/// Whether one instance of [`Permissions`] is a subset of another, the [`contains`] method
/// may be used.
///
/// [`contains`]: Self::contains
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Permissions(pub u64);

impl Permissions {
	/// Determines whether `permissions` is a subset of `self`.
	pub const fn contains(&self, permissions: Self) -> bool {
		self.0 & permissions.0 == permissions.0
	}
}

// Maps - 1..=10
macros::perms! {
	/// Add maps to the global map pool
	MAPS_ADD = 1 << 1;

	/// Edit a global map's attributes, such as its name, tier, ranked status, etc.
	MAPS_EDIT = 1 << 2;

	/// Delete a map from the global map pool
	MAPS_DELETE = 1 << 3;
}

// Servers - 11..=20
macros::perms! {
	/// Invalidate approved servers
	SERVERS_INVALIDATE = 1 << 11;

	/// Approve a server
	SERVERS_ADD = 1 << 12;

	/// Edit a server's attributes, such as its name, IP address, verified status, etc.
	SERVERS_EDIT = 1 << 13;
}

// Bans - 21..=30
macros::perms! {
	/// Ban a player
	BANS_ADD = 1 << 21;

	/// Modify a ban's attributes, such as its duration, reason, etc.
	BANS_EDIT = 1 << 22;
}

// Profiles - 31..=40
macros::perms! {
	/// Edit any profile on `cs2.kz`
	PROFILES_EDIT = 1 << 31;

	/// Lock any profile on `cs2.kz`
	PROFILES_LOCK = 1 << 32;
}

macros::perms! {
	/// Full control over other (non global admins) users' permissions.
	GLOBAL_ADMIN = 1 << 63;
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

mod macros {
	macro_rules! perms {
		(
			$(
				$( #[doc = $docs:literal] )*
				$name:ident = $value:expr;
			)*
		) => {
			impl Permissions {
				$(
					$( #[doc = $docs] )*
					pub const $name: Self = Self($value);
				)*
			}
		};
	}

	pub(super) use perms;
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
		let a = Permissions::MAPS_ADD | Permissions::MAPS_EDIT | Permissions::MAPS_DELETE;
		let b = Permissions::MAPS_EDIT | Permissions::MAPS_DELETE;

		assert!(a.contains(b));
		assert!(!b.contains(a));
	}
}
