use std::fmt;
use std::ops::{BitAnd, BitOr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bitfield for storing user roles.
///
/// See associated constants for details.
#[repr(transparent)]
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct RoleFlags(pub u32);

impl RoleFlags {
	/// Determines whether `self` contains `other`.
	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	/// Creates an iterator over the roles stored in this bitfield.
	pub const fn iter(self) -> RoleIter {
		RoleIter::new(self)
	}

	/// Determines whether the `n`th bit in `self` is set to `1`.
	const fn has_bit(self, n: u32) -> bool {
		(self.0 >> n) & 1 == 1
	}
}

#[allow(dead_code)]
impl RoleFlags {
	pub const NONE: Self = Self(0);
	pub const BANS: Self = Self(1 << 0);
	pub const SERVERS: Self = Self(1 << 8);
	pub const MAPS: Self = Self(1 << 16);
	pub const ADMIN: Self = Self(1 << 31);
	pub const ALL: Self = Self(Self::BANS.0 | Self::SERVERS.0 | Self::MAPS.0 | Self::ADMIN.0);
}

impl BitOr for RoleFlags {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitAnd for RoleFlags {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}

impl IntoIterator for RoleFlags {
	type Item = Role;
	type IntoIter = RoleIter;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl FromIterator<Role> for RoleFlags {
	fn from_iter<T: IntoIterator<Item = Role>>(iter: T) -> Self {
		iter.into_iter()
			.fold(Self::NONE, |acc, curr| acc | Self(curr as u32))
	}
}

#[repr(u32)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
	#[default]
	None = 0,
	Bans = 1 << 0,
	Servers = 1 << 8,
	Maps = 1 << 16,
	Admin = 1 << 31,
}

impl fmt::Display for Role {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl From<Role> for RoleFlags {
	fn from(value: Role) -> Self {
		Self(value as u32)
	}
}

impl From<RoleFlags> for Option<Role> {
	fn from(value: RoleFlags) -> Self {
		match value {
			RoleFlags::NONE => Some(Role::None),
			RoleFlags::BANS => Some(Role::Bans),
			RoleFlags::SERVERS => Some(Role::Servers),
			RoleFlags::MAPS => Some(Role::Maps),
			RoleFlags::ADMIN => Some(Role::Admin),
			_ => None,
		}
	}
}

pub struct RoleIter {
	flags: RoleFlags,
	idx: u32,
}

impl RoleIter {
	pub const fn new(flags: RoleFlags) -> Self {
		Self { flags, idx: 0 }
	}
}

impl Iterator for RoleIter {
	type Item = Role;

	fn next(&mut self) -> Option<Self::Item> {
		if self.idx >= 31 {
			return None;
		}

		while self.idx < 31 && !self.flags.has_bit(self.idx) {
			self.idx += 1;
		}

		if !self.flags.has_bit(self.idx) {
			return None;
		}

		// TODO(AlphaKeks): I hate this.
		let next = match RoleFlags(1_u32 << self.idx) {
			RoleFlags::NONE => Role::None,
			RoleFlags::BANS => Role::Bans,
			RoleFlags::SERVERS => Role::Servers,
			RoleFlags::MAPS => Role::Maps,
			RoleFlags::ADMIN => Role::Admin,
			_ => {
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
	use super::{Role, RoleFlags};

	#[test]
	fn zero() {
		let zero = RoleFlags::default();

		assert_eq!(zero.0, 0);
		assert!(zero.has_bit(zero.0));
	}

	#[test]
	fn one() {
		let maps = RoleFlags::MAPS;
		let zero = RoleFlags::default();

		assert!(!zero.has_bit(maps.0));
		assert!(maps.has_bit(maps.0));
		assert!(maps.has_bit(zero.0));
	}

	#[test]
	fn multiple() {
		let a = RoleFlags::MAPS | RoleFlags::BANS | RoleFlags::SERVERS;
		let b = RoleFlags::BANS | RoleFlags::SERVERS;

		assert!(a.has_bit(b.0));
		assert!(!b.has_bit(a.0));
	}

	#[test]
	fn iter() {
		let perms = RoleFlags::BANS | RoleFlags::MAPS | RoleFlags::ADMIN;
		let perms = perms.iter().collect::<Vec<_>>();

		assert_eq!(perms, [Role::Bans, Role::Maps, Role::Admin,]);
	}
}
