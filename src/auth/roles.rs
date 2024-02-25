//! Roles assigned to authorized users.

use std::fmt::{self, Display};
use std::ops::BitOr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The available roles as a descriptive enum.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
	Bans = 1 << 0,
	Servers = 1 << 8,
	Maps = 1 << 16,
	Admin = 1 << 31,
}

impl Display for Role {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Bans => "bans",
			Self::Servers => "servers",
			Self::Maps => "maps",
			Self::Admin => "admin",
		})
	}
}

/// Bitfield as [`Role`]s are stored in the database.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, ToSchema)]
#[sqlx(transparent)]
pub struct RoleFlags(u32);

impl RoleFlags {
	pub const NONE: Self = Self(0);
	pub const BANS: Self = Self(1 << 0);
	pub const SERVERS: Self = Self(1 << 8);
	pub const MAPS: Self = Self(1 << 16);
	pub const ADMIN: Self = Self(1 << 31);

	/// Checks whether `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	/// Checks whether the `n`th bit in `self` is set.
	pub const fn has_bit(self, n: u32) -> bool {
		(self.0 >> n) & 1 == 1
	}

	/// Creates an iterator over the roles encoded in `self`.
	pub const fn iter(self) -> RoleIter {
		RoleIter::new(self)
	}
}

impl BitOr for RoleFlags {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl From<u32> for RoleFlags {
	fn from(flags: u32) -> Self {
		[Self::BANS, Self::SERVERS, Self::MAPS, Self::ADMIN]
			.into_iter()
			.filter(|&flag| Self(flags).contains(flag))
			.fold(Self::NONE, Self::bitor)
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
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = Role>,
	{
		iter.into_iter()
			.fold(Self::NONE, |flags, role| Self(flags.0 | role as u32))
	}
}

/// Iterator over [`Role`]s.
pub struct RoleIter {
	/// The original flags.
	flags: RoleFlags,

	/// The current bit we are looking at.
	idx: u32,
}

impl RoleIter {
	const fn new(flags: RoleFlags) -> Self {
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

		let role = match RoleFlags::from(1_u32 << self.idx) {
			RoleFlags::BANS => Role::Bans,
			RoleFlags::SERVERS => Role::Servers,
			RoleFlags::MAPS => Role::Maps,
			RoleFlags::ADMIN => Role::Admin,
			_ => unreachable!(),
		};

		self.idx += 1;

		Some(role)
	}
}
