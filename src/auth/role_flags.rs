//! Helper type for dealing with user roles.
//!
//! Every role is represented as a specific bit in a 32-bit integer.
//! If the bit is 1, it means the user has this role.
//!
//! This module contains the [`RoleFlags`] type, which abstracts over this idea.
//! It has various constants defined for the existing roles, and can be serialized / deserialized
//! by serde, as well as be inserted into the database.

use std::fmt::{self, Display};
use std::ops::BitOr;
use std::str::FromStr;

use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Bitfield for holding role information.
///
/// See [module level documentation] for more details.
///
/// [module level documentation]: crate::auth::role_flags
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct RoleFlags(u32);

#[allow(clippy::missing_docs_in_private_items)]
impl RoleFlags {
	pub const NONE: Self = Self(0);
	pub const BANS: Self = Self(1 << 0);
	pub const SERVERS: Self = Self(1 << 8);
	pub const MAPS: Self = Self(1 << 16);
	pub const ADMIN: Self = Self(1 << 31);
}

impl RoleFlags {
	/// Returns the internal 32-bit integer.
	pub const fn as_u32(self) -> u32 {
		self.0
	}

	/// Checks if the `n`th bit is set to 1.
	pub const fn bit(self, n: u32) -> bool {
		(self.0 >> n) & 1 == 1
	}

	/// Checks if `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}
}

impl Display for RoleFlags {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list().entries(RoleIter::new(*self)).finish()
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

#[derive(Debug, Error)]
#[error("unknown role `{0}`")]
#[allow(clippy::missing_docs_in_private_items)]
pub struct UnknownRole(String);

impl FromStr for RoleFlags {
	type Err = UnknownRole;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"bans" => Ok(Self::BANS),
			"servers" => Ok(Self::SERVERS),
			"maps" => Ok(Self::MAPS),
			"admin" => Ok(Self::ADMIN),
			value => Err(UnknownRole(value.to_owned())),
		}
	}
}

impl BitOr for RoleFlags {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl Serialize for RoleFlags {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(None)?;

		for role in RoleIter::new(*self) {
			serializer.serialize_element(role)?;
		}

		serializer.end()
	}
}

impl<'de> Deserialize<'de> for RoleFlags {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		enum Helper {
			Int(u32),
			Words(Vec<String>),
		}

		Helper::deserialize(deserializer).map(|value| match value {
			Helper::Int(flags) => Self::from(flags),
			Helper::Words(words) => words
				.into_iter()
				.flat_map(|word| word.parse::<Self>())
				.fold(Self::NONE, Self::bitor),
		})
	}
}

/// Iterator over [`RoleFlags`] that will produce string representations for every present role.
struct RoleIter {
	/// The flags.
	flags: RoleFlags,

	/// The current bit "index".
	current_bit: u32,
}

impl RoleIter {
	/// Creates a new [`RoleIter`].
	const fn new(flags: RoleFlags) -> Self {
		Self { flags, current_bit: 0 }
	}
}

impl Iterator for RoleIter {
	type Item = &'static str;

	fn next(&mut self) -> Option<Self::Item> {
		if self.current_bit >= 31 {
			return None;
		}

		while self.current_bit < 31 && !self.flags.bit(self.current_bit) {
			self.current_bit += 1;
		}

		if !self.flags.bit(self.current_bit) {
			return None;
		}

		let role = match RoleFlags(1 << self.current_bit) {
			RoleFlags::BANS => "bans",
			RoleFlags::SERVERS => "servers",
			RoleFlags::MAPS => "maps",
			RoleFlags::ADMIN => "admin",
			_ => unreachable!(),
		};

		self.current_bit += 1;

		Some(role)
	}
}
