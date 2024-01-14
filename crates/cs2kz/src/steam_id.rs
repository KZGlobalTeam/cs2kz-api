use std::borrow::Borrow;
use std::str::FromStr;

use derive_more::{AsRef, Deref, Display};

use crate::{Error, Result};

#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AsRef, Deref)]
#[display("STEAM_{}:{}:{}", self.account_universe(), self.account_type(), self.account_number())]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(value_type = String))]
pub struct SteamID(u64);

impl SteamID {
	/// The minimum value for a legal [`SteamID`].
	pub const MIN: Self = Self(76561197960265729_u64);

	/// The maximum value for a legal [`SteamID`].
	pub const MAX: Self = Self(76561202255233023_u64);

	/// Returns a 64-bit representation of the inner SteamID.
	#[inline]
	pub const fn as_u64(&self) -> u64 {
		self.0
	}

	/// Returns a 32-bit representation of the inner SteamID.
	#[inline]
	pub const fn as_u32(&self) -> u32 {
		let account_type = self.account_type() as u32;

		((self.account_number() + account_type) * 2) - account_type
	}

	/// Returns the "Steam3ID" representation of the inner SteamID.
	#[inline]
	pub fn as_id3(&self) -> String {
		format!("U:1:{}", self.as_u32())
	}

	#[inline]
	pub const fn account_universe(&self) -> u64 {
		self.as_u64() >> 56
	}

	#[inline]
	pub const fn account_type(&self) -> u64 {
		self.as_u64() & 1
	}

	/// Returns the `Z` part of `STEAM_X:Y:Z`.
	#[inline]
	pub const fn account_number(&self) -> u32 {
		let account_number = (self.as_u64() - Self::MAGIC_OFFSET - self.account_type()) / 2;

		debug_assert!(account_number <= u32::MAX as u64);

		account_number as u32
	}
}

impl SteamID {
	const MAGIC_OFFSET: u64 = Self::MIN.0 - 1;

	/// Constructs a [`SteamID`] from a 64-bit integer `value`.
	#[inline]
	pub const fn from_u64(value: u64) -> Result<Self> {
		if value < Self::MIN.0 || value > Self::MAX.0 {
			return Err(Error::OutOfBoundsSteamID { value });
		}

		Ok(Self(value))
	}

	/// Constructs a [`SteamID`] from a 32-bit integer `value`.
	#[inline]
	pub const fn from_u32(value: u32) -> Result<Self> {
		if value == 0 {
			return Err(Error::OutOfBoundsSteamID { value: value as u64 });
		}

		let steam_id = value as u64 + Self::MAGIC_OFFSET;

		if steam_id > Self::MAX.0 {
			return Err(Error::OutOfBoundsSteamID { value: value as u64 });
		}

		Ok(Self(steam_id))
	}

	/// Parses a [`SteamID`] as the "Steam3ID" format.
	pub fn from_id3(value: impl AsRef<str>) -> Result<Self> {
		let value = value.as_ref();
		let err = |reason: &'static str| Error::InvalidSteam3ID { value: value.to_owned(), reason };

		let (_, mut steam_id) = value.rsplit_once(':').ok_or_else(|| err("missing `:`"))?;

		if steam_id.ends_with(']') {
			steam_id = &steam_id[..(steam_id.len() - 1)];
		}

		steam_id
			.parse::<u32>()
			.map_err(|_| err("invalid ID part"))
			.and_then(Self::from_u32)
	}

	/// Parses a [`SteamID`] as the "standard" `STEAM_X:Y:Z` format.
	pub fn from_standard(value: impl AsRef<str>) -> Result<Self> {
		let value = value.as_ref();
		let err = |reason: &'static str| Error::InvalidSteamID { value: value.to_owned(), reason };

		let mut segments = value
			.split_once('_')
			.ok_or_else(|| err("missing `_`"))?
			.1
			.split(':');

		// Parse the `X` part.
		//
		// This is always `1` for Counter-Strike, but some websites might display a `0`, so
		// we allow either and just proceed with `1`.
		if !matches!(segments.next(), Some("0" | "1")) {
			return Err(err("invalid `X` segment"));
		}

		// Parse the `Y` part.
		let y = segments
			.next()
			.ok_or_else(|| err("missing `Y` segment"))?
			.parse::<u64>()
			.map_err(|_| err("invalid `Y` segment"))?;

		if y > 1 {
			return Err(err("invalid `Y` segment"));
		}

		// Parse the `Z` part.
		let z = segments
			.next()
			.ok_or_else(|| err("missing `Z` segment"))?
			.parse::<u64>()
			.map_err(|_| err("invalid `Z` segment"))?;

		// At this point we should be done.
		if segments.next().is_some() {
			return Err(err("trailing characters"));
		}

		if y == 0 && z == 0 {
			return Err(err("is 0"));
		}

		if z + Self::MAGIC_OFFSET > Self::MAX.0 {
			return Err(Error::OutOfBoundsSteamID { value: z });
		}

		let steam_id = Self::MAGIC_OFFSET | y | z << 1;

		Self::from_u64(steam_id)
	}
}

impl FromStr for SteamID {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		Self::from_standard(value)
			.or_else(|_| Self::from_id3(value))
			.or_else(|_| {
				if let Ok(value) = value.parse::<u32>() {
					Self::from_u32(value)
				} else if let Ok(value) = value.parse::<u64>() {
					Self::from_u64(value)
				} else {
					Err(Error::InvalidSteamID {
						value: value.to_owned(),
						reason: "unrecognized format",
					})
				}
			})
	}
}

impl TryFrom<u32> for SteamID {
	type Error = Error;

	fn try_from(value: u32) -> Result<Self> {
		Self::from_u32(value)
	}
}

impl From<SteamID> for u32 {
	fn from(value: SteamID) -> Self {
		value.as_u32()
	}
}

impl TryFrom<u64> for SteamID {
	type Error = Error;

	fn try_from(value: u64) -> Result<Self> {
		if value <= u32::MAX as u64 {
			return Self::from_u32(value as u32);
		}

		Self::from_u64(value)
	}
}

impl From<SteamID> for u64 {
	fn from(value: SteamID) -> Self {
		value.as_u64()
	}
}

impl Borrow<u64> for SteamID {
	#[inline]
	fn borrow(&self) -> &u64 {
		&self.0
	}
}

impl PartialEq<u64> for SteamID {
	fn eq(&self, other: &u64) -> bool {
		&self.0 == other
	}
}

impl PartialEq<u32> for SteamID {
	fn eq(&self, other: &u32) -> bool {
		self.as_u32() == *other
	}
}

#[cfg(test)]
mod tests {
	use super::SteamID;

	#[test]
	fn it_works() {
		let steam_id = SteamID(76561198282622073_u64);
		let u32 = 322356345_u32;
		let u64 = 76561198282622073_u64;
		let id3_1 = "U:1:322356345";
		let id3_2 = "[U:1:322356345]";
		let standard = "STEAM_1:1:161178172";

		assert_eq!(SteamID::from_u32(u32), Ok(steam_id));
		assert_eq!(SteamID::from_u64(u64), Ok(steam_id));
		assert_eq!(SteamID::from_id3(id3_1), Ok(steam_id));
		assert_eq!(SteamID::from_id3(id3_2), Ok(steam_id));
		assert_eq!(SteamID::from_standard(standard), Ok(steam_id));
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::SteamID;
	use crate::serde::IntOrStr;

	impl SteamID {
		/// Serializes the given `steam_id` using the standard `STEAM_X:Y:Z` format.
		pub fn serialize_standard<S: Serializer>(
			steam_id: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			steam_id.to_string().serialize(serializer)
		}

		/// Serializes the given `steam_id` using the "Steam3ID" format.
		pub fn serialize_id3<S: Serializer>(
			steam_id: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			steam_id.as_id3().serialize(serializer)
		}

		/// Serializes the given `steam_id` as a 32-bit integer.
		pub fn serialize_u32<S: Serializer>(
			steam_id: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			steam_id.as_u32().serialize(serializer)
		}

		/// Serializes the given `steam_id` as a 64-bit integer.
		pub fn serialize_u64<S: Serializer>(
			steam_id: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			steam_id.as_u64().serialize(serializer)
		}
	}

	impl Serialize for SteamID {
		/// By default [`SteamID::serialize_standard`] is used for serialization, but you
		/// can use any of the `serialize_*` functions and pass them to
		/// `#[serde(serialize_with = "...")]` if you need a different method.
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			Self::serialize_standard(self, serializer)
		}
	}

	impl SteamID {
		/// Deserializes a string in the standard `STEAM_X:Y:Z` format into a [`SteamID`].
		pub fn deserialize_standard<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			let value = <&str as Deserialize>::deserialize(deserializer)?;

			Self::from_standard(value).map_err(serde::de::Error::custom)
		}

		/// Deserializes a string in the "Steam3ID" format into a [`SteamID`].
		pub fn deserialize_id3<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			let value = <&str as Deserialize>::deserialize(deserializer)?;

			Self::from_id3(value).map_err(serde::de::Error::custom)
		}

		/// Deserializes a 32-bit integer into a [`SteamID`].
		pub fn deserialize_u32<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			let value = <u32 as Deserialize>::deserialize(deserializer)?;

			Self::from_u32(value).map_err(serde::de::Error::custom)
		}

		/// Deserializes a 64-bit integer into a [`SteamID`].
		pub fn deserialize_u64<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			let value = <u64 as Deserialize>::deserialize(deserializer)?;

			Self::from_u64(value).map_err(serde::de::Error::custom)
		}
	}

	impl<'de> Deserialize<'de> for SteamID {
		/// The default [`Deserialize`] implementation is a best-effort.
		///
		/// This means it considers as many cases as possible; if you want / need
		/// a specific format, consider using `#[serde(deserialize_with = "...")]` in
		/// combination with any of the `deserialize_*` methods on [`SteamID`].
		fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			match <IntOrStr<u64> as Deserialize<'de>>::deserialize(deserializer)? {
				IntOrStr::Int(value) if value <= (u32::MAX as u64) => Self::from_u32(value as u32),
				IntOrStr::Int(value) => Self::from_u64(value),
				IntOrStr::Str(value) => value.parse(),
			}
			.map_err(serde::de::Error::custom)
		}
	}
}

#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use super::SteamID;

	crate::sqlx::from_row_as!(SteamID as u32 {
		encode: |steam_id| { steam_id.as_u32() }
		decode: |int| { SteamID::from_u32(int) }
	});
}

#[cfg(feature = "utoipa")]
crate::utoipa::into_params!(SteamID as "steam_id": "");
