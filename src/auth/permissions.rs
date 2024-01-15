use std::ops::{BitAnd, BitOr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bitfield for storing user permissions.
///
/// See associated constants for details.
#[repr(transparent)]
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Permissions(pub u64);

impl Permissions {
	/// Determines whether `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	pub const fn nth_bit(self, n: u64) -> bool {
		(self.0 >> n) & 1 == 1
	}

	pub const fn iter(self) -> PermissionsIter {
		PermissionsIter::new(self)
	}
}

#[allow(dead_code)]
impl Permissions {
	pub const NONE: Self = Self(u64::MIN);
	pub const ALL: Self = Self(u64::MAX);

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

	pub const MANAGE_ADMINS: Self = Self(1 << 63);
}

impl From<u64> for Permissions {
	fn from(value: u64) -> Self {
		Self(value)
	}
}

impl From<Permissions> for u64 {
	fn from(value: Permissions) -> Self {
		value.0
	}
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

impl IntoIterator for Permissions {
	type Item = Permission;
	type IntoIter = PermissionsIter;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl FromIterator<Permission> for Permissions {
	fn from_iter<T: IntoIterator<Item = Permission>>(iter: T) -> Self {
		iter.into_iter()
			.fold(Self::NONE, |acc, curr| acc | Self(curr as u64))
	}
}

#[repr(u64)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
	#[default]
	None = u64::MIN,
	All = u64::MAX,

	MapsView = 1 << 0,
	MapsApprove = 1 << 1,
	MapsEdit = 1 << 2,
	MapsDeglobal = 1 << 3,

	ServersApprove = 1 << 10,
	ServersEdit = 1 << 11,
	ServersDeglobal = 1 << 12,

	BansCreate = 1 << 20,
	BansEdit = 1 << 21,
	BansRemove = 1 << 22,

	ManageAdmins = 1 << 63,
}

pub struct PermissionsIter {
	permissions: Permissions,
	idx: u64,
}

impl PermissionsIter {
	const fn new(permissions: Permissions) -> Self {
		Self { permissions, idx: 0 }
	}
}

impl Iterator for PermissionsIter {
	type Item = Permission;

	fn next(&mut self) -> Option<Self::Item> {
		if self.idx >= 63 {
			return None;
		}

		while self.idx < 63 && !self.permissions.nth_bit(self.idx) {
			self.idx += 1;
		}

		if !self.permissions.nth_bit(self.idx) {
			return None;
		}

		// TODO(AlphaKeks): I hate this.
		let next = match Permissions(1_u64 << self.idx) {
			Permissions::NONE => Permission::None,
			Permissions::ALL => Permission::All,
			Permissions::MAPS_VIEW => Permission::MapsView,
			Permissions::MAPS_APPROVE => Permission::MapsApprove,
			Permissions::MAPS_EDIT => Permission::MapsEdit,
			Permissions::MAPS_DEGLOBAL => Permission::MapsDeglobal,
			Permissions::SERVERS_APPROVE => Permission::ServersApprove,
			Permissions::SERVERS_EDIT => Permission::ServersEdit,
			Permissions::SERVERS_DEGLOBAL => Permission::ServersDeglobal,
			Permissions::BANS_CREATE => Permission::BansCreate,
			Permissions::BANS_EDIT => Permission::BansEdit,
			Permissions::BANS_REMOVE => Permission::BansRemove,
			Permissions::MANAGE_ADMINS => Permission::ManageAdmins,
			_ => {
				// Unused bit.
				self.idx += 1;
				return self.next();
			}
		};

		self.idx += 1;

		Some(next)
	}
}

#[cfg(test)]
mod tests {
	use super::{Permission, Permissions};

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

	#[test]
	fn iter() {
		let perms = Permissions::BANS_EDIT | Permissions::MAPS_APPROVE | Permissions::MAPS_EDIT;
		let perms = perms.iter().collect::<Vec<_>>();

		assert_eq!(perms, [
			Permission::MapsApprove,
			Permission::MapsEdit,
			Permission::BansEdit
		]);
	}
}
