use std::ops::{BitAnd, BitOr};

use serde::{Deserialize, Serialize, Serializer};
use utoipa::ToSchema;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
	Bans = 1 << 0,
	Servers = 1 << 8,
	Maps = 1 << 16,
	Admin = 1 << 31,
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, ToSchema)]
#[sqlx(transparent)]
pub struct RoleFlags(pub u32);

impl RoleFlags {
	pub const NONE: Self = Self(0);
	pub const BANS: Self = Self(1 << 0);
	pub const SERVERS: Self = Self(1 << 8);
	pub const MAPS: Self = Self(1 << 16);
	pub const ADMIN: Self = Self(1 << 31);
	pub const ALL: Self = Self(Self::BANS.0 | Self::SERVERS.0 | Self::MAPS.0 | Self::ADMIN.0);

	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	pub const fn iter(&self) -> RoleIter {
		RoleIter::new(*self)
	}

	pub fn for_subdomain(subdomain: &str) -> Self {
		match subdomain {
			"dashboard" => Self::BANS | Self::SERVERS | Self::MAPS | Self::ADMIN,
			_ => Self::default(),
		}
	}
}

impl BitOr for RoleFlags {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl BitOr<u32> for RoleFlags {
	type Output = Self;

	fn bitor(self, rhs: u32) -> Self::Output {
		Self(self.0 | rhs)
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
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = Role>,
	{
		iter.into_iter()
			.fold(Self::default(), |flags, role| flags | role as u32)
	}
}

impl Serialize for RoleFlags {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.into_iter().collect::<Vec<_>>().serialize(serializer)
	}
}

pub struct RoleIter {
	flags: RoleFlags,
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

		let has_bit = |n| (self.flags.0 >> n) & 1 == 1;

		while self.idx < 31 && !has_bit(self.idx) {
			self.idx += 1;
		}

		if !has_bit(self.idx) {
			return None;
		}

		let role = match RoleFlags(1_u32 << self.idx) {
			RoleFlags::NONE => unreachable!(),
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

		Some(role)
	}
}
