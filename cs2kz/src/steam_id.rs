// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{Error, Result},
	lazy_regex::{regex, Lazy, Regex},
	std::{
		borrow::Borrow,
		cmp,
		fmt::Display,
		hash::{Hash, Hasher},
		ops::Deref,
		str::FromStr,
	},
	utoipa::ToSchema,
};

/// A regex to match [`SteamID`]s in the format of `STEAM_1:1:161178172`.
pub static STANDARD_REGEX: &Lazy<Regex> = regex!(r"^STEAM_[01]:[01]:\d+$");

/// A regex to match [`SteamID`]s in the format of `U:1:322356345` or `[U:1:322356345]`.
pub static STEAM3_ID_REGEX: &Lazy<Regex> = regex!(r"^(\[U:1:\d+\]|U:1:\d+)$");

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ToSchema)]
pub struct SteamID(u64);

#[rustfmt::skip]
impl SteamID {
	pub const MIN: u64 = 76561197960265729_u64;
	pub const MAX: u64 = 76561202255233023_u64;
	pub const MAGIC_OFFSET: u64 = Self::MIN - 1_u64;
	pub const ACCOUNT_UNIVERSE: u64 = 1;
	pub const ACCOUNT_TYPE: u64 = 1;

	// 4294967296 (0x100_000_000)
	const MSB_9_BITS: u64 = 1 << 32;

	// 4503599627370496 (0x010_000_000_000_000)
	const MSB_14_BITS: u64 = 1 << 52;
}

macro_rules! invalid {
	($input:expr) => {
		return Err($crate::Error::InvalidSteamID {
			input: $input.to_string(),
			reason: None,
		})
	};

	($input:expr, $($t:tt)*) => {
		return Err($crate::Error::InvalidSteamID {
			input: $input.to_string(),
			reason: Some(format!($($t)*)),
		})
	};
}

impl SteamID {
	pub fn new<S>(input: S) -> Result<Self>
	where
		S: AsRef<str>, {
		let input = input.as_ref();

		if STANDARD_REGEX.is_match(input) {
			Self::from_standard(input)
		} else if STEAM3_ID_REGEX.is_match(input) {
			Self::from_id3(input)
		} else if let Ok(int) = input.parse::<u64>() {
			Self::try_from(int)
		} else {
			invalid!(input);
		}
	}

	#[inline]
	pub const fn as_u64(&self) -> u64 {
		self.0
	}

	#[inline]
	pub const fn as_u32(&self) -> u32 {
		let account_number = self.account_number();
		let account_type = self.account_type() as u32;

		((account_number + account_type) * 2) - account_type
	}

	#[inline]
	pub fn as_steam3_id(&self, brackets: bool) -> String {
		let id32 = self.as_u32();

		if brackets { format!("[U:1:{id32}]") } else { format!("U:1:{id32}") }
	}

	#[inline]
	pub const fn account_universe(&self) -> u64 {
		Self::ACCOUNT_UNIVERSE
	}

	#[inline]
	pub const fn account_type(&self) -> u64 {
		Self::ACCOUNT_TYPE
	}

	#[inline]
	pub const fn lsb(&self) -> u64 {
		self.as_u64() & 1
	}

	#[inline]
	pub const fn account_number(&self) -> u32 {
		let offset = self.as_u64() - Self::MAGIC_OFFSET;
		let account_type = self.account_type();
		let account_number = (offset - account_type) / 2;

		assert!(account_number <= u32::MAX as u64);

		account_number as u32
	}
}

impl SteamID {
	pub fn from_standard<S>(input: S) -> Result<Self>
	where
		S: AsRef<str>, {
		let input = input.as_ref();

		if !STANDARD_REGEX.is_match(input) {
			invalid!(input, "does not match \"standard\" regex");
		}

		assert!(input.is_ascii(), "SteamID must be valid ASCII.");

		// (X, Y, Z) parts of STEAM_X:Y:Z
		//
		// Assuming we start with "STEAM_1:1:161178172":
		let mut segments = input
			.split_once('_') // ("STEAM", "1:1:161178172")
			.expect("SteamID contains an underscore.")
			.1 // "1:1:161178172"
			.split(':'); // ("1", "1", "16117817")

		// `X` is always 0 or 1.
		//
		// For Counter-Strike this is really always 1, but websites might give players their
		// SteamID with a 0 in the first position, so we just make sure it's either one and
		// proceed with 1.
		let account_universe = {
			assert!(matches!(segments.next(), Some("0" | "1")));
			Self::ACCOUNT_UNIVERSE
		};

		// `Y` is also always 0 or 1.
		let y_bit = segments
			.next()
			.expect("There are two segments left.")
			.parse::<u64>()
			.expect("The second segment is either 0 or 1.");

		assert!(y_bit < 2, "The `Y` bit should always be either 0 or 1.");

		// `Z` part
		let account_number = segments
			.next()
			.expect("There is a single segment left.")
			.parse::<u64>()
			.expect("The last segment is an integer.");

		// At this point we should be done.
		assert!(segments.next().is_none(), "There should be no more segments left.");

		if y_bit == 0 && account_number == 0 {
			invalid!(input, "is 0");
		}

		if account_number + Self::MAGIC_OFFSET > Self::MAX {
			invalid!(input, "too large");
		}

		let steam64_id = account_universe << 56
			| Self::MSB_14_BITS
			| Self::MSB_9_BITS
			| account_number << 1
			| y_bit;

		assert!((Self::MIN..=Self::MAX).contains(&steam64_id));

		Ok(Self(steam64_id))
	}

	pub fn from_id3<S>(input: S) -> Result<Self>
	where
		S: AsRef<str>, {
		let input = input.as_ref();

		if !STEAM3_ID_REGEX.is_match(input) {
			invalid!(input, "does not match \"Steam3 ID\" regex");
		}

		assert!(input.is_ascii(), "SteamID must be valid ASCII.");

		let (_, mut id32) = input
			.rsplit_once(':')
			.expect("Steam3 ID always contains a `:`.");

		if id32.ends_with(']') {
			id32 = &id32[..(id32.len() - 1)];
		}

		let steam32_id = id32
			.parse::<u32>()
			.expect("Only digits are left.");

		Self::try_from(steam32_id)
	}

	#[inline]
	pub fn from_id32(input: u32) -> Result<Self> {
		let steam64_id = input as u64 + Self::MAGIC_OFFSET;

		if steam64_id > Self::MAX {
			invalid!(steam64_id, "too large");
		}

		Ok(Self(steam64_id))
	}

	#[inline]
	pub fn from_id64(input: u64) -> Result<Self> {
		let allowed_range = Self::MIN..=Self::MAX;

		if !allowed_range.contains(&input) {
			invalid!(input, "out of bounds");
		}

		Ok(Self(input))
	}
}

impl TryFrom<u32> for SteamID {
	type Error = Error;

	fn try_from(input: u32) -> Result<Self> {
		Self::from_id32(input)
	}
}

impl TryFrom<i32> for SteamID {
	type Error = Error;

	fn try_from(input: i32) -> Result<Self> {
		if let Ok(input) = u32::try_from(input) {
			Self::try_from(input)
		} else {
			invalid!(input, "is negative");
		}
	}
}

impl TryFrom<u64> for SteamID {
	type Error = Error;

	fn try_from(input: u64) -> Result<Self> {
		if let Ok(id32) = u32::try_from(input) {
			Self::try_from(id32)
		} else {
			Self::from_id64(input)
		}
	}
}

impl TryFrom<i64> for SteamID {
	type Error = Error;

	fn try_from(input: i64) -> Result<Self> {
		if let Ok(id32) = u32::try_from(input) {
			Self::try_from(id32)
		} else if let Ok(id64) = u64::try_from(input) {
			Self::from_id64(id64)
		} else {
			invalid!(input, "is negative");
		}
	}
}

impl TryFrom<u128> for SteamID {
	type Error = Error;

	fn try_from(input: u128) -> Result<Self> {
		if let Ok(id64) = u64::try_from(input) {
			Self::from_id64(id64)
		} else {
			invalid!(input, "too large");
		}
	}
}

impl TryFrom<i128> for SteamID {
	type Error = Error;

	fn try_from(input: i128) -> Result<Self> {
		if let Ok(id64) = u64::try_from(input) {
			Self::from_id64(id64)
		} else {
			invalid!(input, "out of bounds");
		}
	}
}

impl TryFrom<&str> for SteamID {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for SteamID {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for SteamID {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		Self::new(input)
	}
}

impl From<SteamID> for u64 {
	fn from(steam_id: SteamID) -> Self {
		steam_id.as_u64()
	}
}

impl From<SteamID> for u32 {
	fn from(steam_id: SteamID) -> Self {
		steam_id.as_u32()
	}
}

impl PartialEq<u64> for SteamID {
	fn eq(&self, other: &u64) -> bool {
		&self.as_u64() == other
	}
}

impl PartialEq<u32> for SteamID {
	fn eq(&self, other: &u32) -> bool {
		&self.as_u32() == other
	}
}

impl PartialOrd<u64> for SteamID {
	fn partial_cmp(&self, other: &u64) -> Option<cmp::Ordering> {
		self.as_u64().partial_cmp(other)
	}
}

impl PartialOrd<u32> for SteamID {
	fn partial_cmp(&self, other: &u32) -> Option<cmp::Ordering> {
		self.as_u32().partial_cmp(other)
	}
}

impl Hash for SteamID {
	fn hash<H>(&self, hasher: &mut H)
	where
		H: Hasher, {
		self.0.hash(hasher);
	}
}

impl Borrow<u64> for SteamID {
	fn borrow(&self) -> &u64 {
		&self.0
	}
}

impl Deref for SteamID {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Display for SteamID {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "STEAM_{}:{}:{}", self.account_universe(), self.lsb(), self.account_number())
	}
}

mod serde_impls {
	use {
		super::SteamID,
		serde::{Deserialize, Deserializer, Serialize, Serializer},
	};

	macro_rules! serialize {
		($name:ident, $name_opt:ident, | $steam_id:ident | $impl:block) => {
			/// Helper function for use with [`serde`].
			///
			/// You can pass this function to the `#[serde(serialize_with = "...")]`
			/// attribute to control how a [`SteamID`] will be serialized.
			///
			/// By default the [`serialize_standard`](SteamID::serialize_standard)
			/// function will be used.
			pub fn $name<S: ::serde::Serializer>(
				$steam_id: &Self,
				serializer: S,
			) -> Result<S::Ok, S::Error> {
				use ::serde::Serialize as _;
				($impl).serialize(serializer)
			}

			/// Helper function for use with [`serde`].
			///
			/// You can pass this function to the `#[serde(serialize_with = "...")]`
			/// attribute to control how an [`Option<SteamID>`] will be serialized.
			///
			/// By default the
			/// [`serialize_standard_opt`](SteamID::serialize_standard_opt)
			/// function will be used.
			pub fn $name_opt<S: ::serde::Serializer>(
				$steam_id: &Option<Self>,
				serializer: S,
			) -> Result<S::Ok, S::Error> {
				use ::serde::Serialize as _;
				$steam_id
					.map(|$steam_id| $impl)
					.serialize(serializer)
			}
		};
	}

	#[rustfmt::skip]
	impl SteamID {
		serialize!(serialize_standard, serialize_standard_opt, |steam_id| {
			steam_id.to_string()
		});

		serialize!(serialize_id3, serialize_id3_opt, |steam_id| {
			steam_id.as_steam3_id(false)
		});

		serialize!(serialize_id3_with_brackets, serialize_id3_with_brackets_opt, |steam_id| {
			steam_id.as_steam3_id(true)
		});

		serialize!(serialize_u32, serialize_u32_opt, |steam_id| {
			steam_id.as_u32()
		});

		serialize!(serialize_u64, serialize_u64_opt, |steam_id| {
			steam_id.as_u64()
		});
	}

	impl Serialize for SteamID {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			Self::serialize_standard(self, serializer)
		}
	}

	#[derive(Deserialize)]
	#[serde(untagged)]
	enum Deserializable {
		U32(u32),
		U64(u64),
		String(String),
	}

	impl<'de> Deserialize<'de> for SteamID {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			match Deserializable::deserialize(deserializer)? {
				Deserializable::U32(id32) => Self::try_from(id32),
				Deserializable::U64(id64) => Self::try_from(id64),
				Deserializable::String(input) => Self::new(input),
			}
			.map_err(serde::de::Error::custom)
		}
	}
}

#[cfg(test)]
mod tests {
	use {super::*, color_eyre::Result};

	macro_rules! case {
		(AlphaKeks $name:ident, $input:expr) => {
			case!(__internal, SteamID(76561198282622073_u64), $name, $input);
		};

		(MIN $name:ident, $input:expr) => {
			case!(__internal, SteamID(SteamID::MIN), $name, $input);
		};

		(__internal, $cmp:expr, $name:ident, $input:expr) => {
			#[test]
			fn $name() -> Result<()> {
				assert_eq!($cmp, $input?);
				Ok(())
			}
		};
	}

	mod alphakeks {
		use super::*;

		case!(AlphaKeks from_u64, SteamID::try_from(76561198282622073_u64));
		case!(AlphaKeks from_u32, SteamID::try_from(322356345_u32));
		case!(AlphaKeks from_standard_0, SteamID::new("STEAM_0:1:161178172"));
		case!(AlphaKeks from_standard_1, SteamID::new("STEAM_1:1:161178172"));
		case!(AlphaKeks from_id3, SteamID::new("U:1:322356345"));
		case!(AlphaKeks from_id3_brackets, SteamID::new("[U:1:322356345]"));
	}

	mod min {
		use super::*;

		case!(MIN from_u64, SteamID::try_from(76561197960265728_u64));
		case!(MIN from_u32, SteamID::try_from(1_u32));
		case!(MIN from_standard_0, SteamID::new("STEAM_0:1:0"));
		case!(MIN from_standard_1, SteamID::new("STEAM_1:1:0"));
		case!(MIN from_id3, SteamID::new("U:1:1"));
		case!(MIN from_id3_brackets, SteamID::new("[U:1:1]"));
	}
}
