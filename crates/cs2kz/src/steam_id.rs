//! A type for working with [SteamID]s.

use std::borrow::Borrow;
use std::fmt::{self, Display};
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Deref;
use std::str::FromStr;
use std::{cmp, mem};

use crate::{Error, Result};

/// Wrapper for a [SteamID].
///
/// This is a unique identifier for Steam accounts. Therefore it is used to identify KZ players,
/// across various API boundaries. This type will take care of validation and provides various
/// utility methods you might find useful.
///
/// [SteamID]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZeroU64);

impl SteamID {
	/// The minimum value for a valid [`SteamID`].
	pub const MIN: Self = Self(match NonZeroU64::new(76561197960265729_u64) {
		None => unreachable!(),
		Some(steam_id) => steam_id,
	});

	/// The maximum value for a valid [`SteamID`].
	pub const MAX: Self = Self(match NonZeroU64::new(76561202255233023_u64) {
		None => unreachable!(),
		Some(steam_id) => steam_id,
	});

	/// Used for various calculations.
	const MAGIC_OFFSET: u64 = Self::MIN.as_u64() - 1;

	/// The 64-bit representation.
	#[inline]
	pub const fn as_u64(&self) -> u64 {
		self.0.get()
	}

	/// The 32-bit representation.
	#[inline]
	pub const fn as_u32(&self) -> u32 {
		(((self.z() + self.y()) * 2) - self.y()) as u32
	}

	/// The `X` part in `STEAM_X:Y:Z`.
	///
	/// This is always 0 or 1.
	#[inline]
	pub const fn x(&self) -> u64 {
		self.as_u64() >> 56
	}

	/// The `Y` part in `STEAM_X:Y:Z`.
	///
	/// This is always 0 or 1.
	#[inline]
	pub const fn y(&self) -> u64 {
		self.as_u64() & 1
	}

	/// The `Z` part in `STEAM_X:Y:Z`.
	#[inline]
	pub const fn z(&self) -> u64 {
		(self.as_u64() - Self::MAGIC_OFFSET - self.y()) / 2
	}

	/// The "Steam3ID" representation.
	pub fn as_id3(&self) -> String {
		format!("U:1:{}", self.as_u32())
	}

	/// Best-effort attempt at parsing a [SteamID] from an arbitrary string.
	///
	/// If you know / expect a specific format, you should use a more specific constructor.
	#[inline]
	pub fn new(value: &str) -> Result<Self> {
		value.parse()
	}

	/// Parse a 64-bit [SteamID].
	///
	/// Example value: `76561198282622073`
	#[inline]
	pub const fn from_u64(value: u64) -> Result<Self> {
		if Self::MIN.as_u64() <= value && value <= Self::MAX.as_u64() {
			// SAFETY: the bounds checks above ensure `value` is not zero
			Ok(Self(unsafe { NonZeroU64::new_unchecked(value) }))
		} else {
			Err(Error::InvalidSteamID { reason: "value out of bounds" })
		}
	}

	/// Parse a 32-bit [SteamID].
	///
	/// Example value: `322356345`
	#[inline]
	pub const fn from_u32(value: u32) -> Result<Self> {
		Self::from_u64((value as u64) + Self::MAGIC_OFFSET)
	}

	/// Parse a "Steam3ID" [SteamID].
	///
	/// Example value: `U:1:322356345`
	pub fn from_id3(value: &str) -> Result<Self> {
		value
			.rsplit_once(':')
			.map(|(_, value)| value.trim_end_matches(']'))
			.ok_or(Error::InvalidSteamID { reason: "missing `:`" })?
			.parse::<u32>()
			.map_err(|_| Error::InvalidSteamID { reason: "invalid Steam3ID" })
			.and_then(Self::from_u32)
	}

	/// Parse a "standard" `STEAM_X:Y:Z` [SteamID].
	///
	/// Example value: `STEAM_1:1:161178172`
	pub fn from_standard(value: &str) -> Result<Self> {
		let mut segments = value
			.split_once('_')
			.ok_or(Error::InvalidSteamID { reason: "missing `_`" })?
			.1
			.split(':');

		let Some("0" | "1") = segments.next() else {
			return Err(Error::InvalidSteamID { reason: "invalid `X` segment" });
		};

		let y = segments
			.next()
			.ok_or(Error::InvalidSteamID { reason: "missing `Y` segment" })?
			.parse::<u64>()
			.map_err(|_| Error::InvalidSteamID { reason: "invalid `Y` segment" })?;

		if y > 1 {
			return Err(Error::InvalidSteamID { reason: "invalid `Y` segment" });
		}

		let z = segments
			.next()
			.ok_or(Error::InvalidSteamID { reason: "missing `Z` segment" })?
			.parse::<u64>()
			.map_err(|_| Error::InvalidSteamID { reason: "invalid `Z` segment" })?;

		if y == 0 && z == 0 {
			return Err(Error::InvalidSteamID { reason: "cannot be 0" });
		}

		if (z + Self::MAGIC_OFFSET) > Self::MAX {
			return Err(Error::InvalidSteamID { reason: "value out of bounds" });
		}

		Self::from_u64(Self::MAGIC_OFFSET | y | (z << 1))
	}
}

impl Display for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
	}
}

impl PartialEq<u64> for SteamID {
	fn eq(&self, other: &u64) -> bool {
		self.as_u64().eq(other)
	}
}

impl PartialEq<SteamID> for u64 {
	fn eq(&self, other: &SteamID) -> bool {
		self.eq(&other.as_u64())
	}
}

impl PartialEq<u32> for SteamID {
	fn eq(&self, other: &u32) -> bool {
		self.as_u32().eq(other)
	}
}

impl PartialEq<SteamID> for u32 {
	fn eq(&self, other: &SteamID) -> bool {
		self.eq(&other.as_u32())
	}
}

impl PartialOrd<u64> for SteamID {
	fn partial_cmp(&self, other: &u64) -> Option<cmp::Ordering> {
		self.as_u64().partial_cmp(other)
	}
}

impl PartialOrd<SteamID> for u64 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		self.partial_cmp(&other.as_u64())
	}
}

impl PartialOrd<u32> for SteamID {
	fn partial_cmp(&self, other: &u32) -> Option<cmp::Ordering> {
		self.as_u32().partial_cmp(other)
	}
}

impl PartialOrd<SteamID> for u32 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		self.partial_cmp(&other.as_u32())
	}
}

impl AsRef<NonZeroU64> for SteamID {
	fn as_ref(&self) -> &NonZeroU64 {
		&self.0
	}
}

impl AsRef<u64> for SteamID {
	fn as_ref(&self) -> &u64 {
		// SAFETY:
		//   - `NonZeroU64` is `#[repr(transparent)]`
		//   - we never provide mutable access to the underlying `u64`, so we do not violate
		//     any of `NonZeroU64`'s invariants
		unsafe { mem::transmute(&self.0) }
	}
}

impl Borrow<NonZeroU64> for SteamID {
	fn borrow(&self) -> &NonZeroU64 {
		&self.0
	}
}

impl Borrow<u64> for SteamID {
	fn borrow(&self) -> &u64 {
		// SAFETY:
		//   - `NonZeroU64` is `#[repr(transparent)]`
		//   - we never provide mutable access to the underlying `u64`, so we do not violate
		//     any of `NonZeroU64`'s invariants
		unsafe { mem::transmute(&self.0) }
	}
}

impl Deref for SteamID {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		// SAFETY:
		//   - `NonZeroU64` is `#[repr(transparent)]`
		//   - we never provide mutable access to the underlying `u64`, so we do not violate
		//     any of `NonZeroU64`'s invariants
		unsafe { mem::transmute(&self.0) }
	}
}

impl TryFrom<u64> for SteamID {
	type Error = Error;

	fn try_from(value: u64) -> Result<Self> {
		if let Ok(value) = u32::try_from(value) {
			Self::from_u32(value)
		} else {
			Self::from_u64(value)
		}
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

impl From<SteamID> for NonZeroU32 {
	fn from(value: SteamID) -> Self {
		value.as_u32().try_into().expect("cannot be 0")
	}
}

impl From<SteamID> for u64 {
	fn from(value: SteamID) -> Self {
		value.as_u64()
	}
}

impl From<SteamID> for NonZeroU64 {
	fn from(value: SteamID) -> Self {
		value.0
	}
}

impl FromStr for SteamID {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(steam_id) = Self::from_standard(value) {
			return Ok(steam_id);
		}

		if let Ok(steam_id) = Self::from_id3(value) {
			return Ok(steam_id);
		}

		if let Ok(value) = value.parse::<u32>() {
			return Self::from_u32(value);
		}

		if let Ok(value) = value.parse::<u64>() {
			return Self::from_u64(value);
		}

		Err(Error::InvalidSteamID { reason: "unrecognized format" })
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::SteamID;

		impl SteamID {
			/// Serialize `self` in the "standard" `STEAM_X:Y:Z` format.
			pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.to_string().serialize(serializer)
			}

			/// Serialize `self` in the "Steam3ID" `U:1:XXXXXXXXX` format.
			pub fn serialize_id3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.as_id3().serialize(serializer)
			}

			/// Serialize `self` in the 64-bit format.
			pub fn serialize_id64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.as_u64().serialize(serializer)
			}

			/// Serialize `self` in the 32-bit format.
			pub fn serialize_id32<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.as_u32().serialize(serializer)
			}
		}

		impl Serialize for SteamID {
			/// Uses the [`SteamID::serialize_standard()`] method.
			///
			/// If you need a different format, consider using
			/// `#[serde(serialize_with = "…")]` with one of the other available
			/// `serialize_*` methods.
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.serialize_standard(serializer)
			}
		}
	}

	mod de {
		use serde::de::{Error, Unexpected as U};
		use serde::{Deserialize, Deserializer};

		use crate::SteamID;

		impl SteamID {
			/// Deserializes `STEAM_X:Y:Z` into a [`SteamID`].
			pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let value = <&'de str as Deserialize>::deserialize(deserializer)?;

				Self::from_standard(value).map_err(|_| {
					Error::invalid_value(U::Str(value), &"SteamID in `STEAM_X:Y:Z` format")
				})
			}

			/// Deserializes `U:1:XXXXXXXXX` into a [`SteamID`].
			pub fn deserialize_id3<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let value = <&'de str as Deserialize>::deserialize(deserializer)?;

				Self::from_id3(value).map_err(|_| {
					Error::invalid_value(U::Str(value), &"SteamID in `U:1:XXXXXXXXX` format")
				})
			}

			/// Deserializes a 64-bit integer into a [`SteamID`].
			pub fn deserialize_id64<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let value = u64::deserialize(deserializer)?;

				Self::from_u64(value)
					.map_err(|_| Error::invalid_value(U::Unsigned(value), &"64-bit SteamID"))
			}

			/// Deserializes 32-bit integer into a [`SteamID`].
			pub fn deserialize_id32<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				let value = u32::deserialize(deserializer)?;

				Self::from_u32(value)
					.map_err(|_| Error::invalid_value(U::Unsigned(value as u64), &"32-bit SteamID"))
			}
		}

		impl<'de> Deserialize<'de> for SteamID {
			/// Best-effort attempt at deserializing a [`SteamID`] of unknown format.
			///
			/// If you know / expect the specific format, consider using
			/// `#[serde(deserialize_with = "…")]` with one of the `deserialize_*`
			/// methods instead.
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Deserialize)]
				#[serde(untagged)]
				enum Helper<'a> {
					U32(u32),
					U64(u64),
					Str(&'a str),
				}

				match <Helper<'de>>::deserialize(deserializer)? {
					Helper::U32(value) => Self::try_from(value),
					Helper::U64(value) => Self::try_from(value),
					Helper::Str(value) => value.parse::<Self>(),
				}
				.map_err(Error::custom)
			}
		}
	}
}

#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use sqlx::database::{HasArguments, HasValueRef};
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use crate::SteamID;

	impl<DB> Type<DB> for SteamID
	where
		DB: Database,
		u64: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u64 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for SteamID
	where
		DB: Database,
		u64: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			self.as_u64().encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for SteamID
	where
		DB: Database,
		u64: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			u64::decode(value).map(Self::from_u64)?.map_err(Into::into)
		}
	}
}

#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::schema::{AnyOfBuilder, Schema};
	use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::SteamID;

	impl<'s> ToSchema<'s> for SteamID {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"SteamID",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.description(Some("a player's SteamID"))
						.example(Some("STEAM_1:1:161178172".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Steam ID"))
								.schema_type(SchemaType::String)
								.example(Some("STEAM_1:1:161178172".into()))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Steam ID3"))
								.schema_type(SchemaType::String)
								.example(Some("U:1:322356345".into()))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Steam ID32"))
								.schema_type(SchemaType::Integer)
								.example(Some(322356345_u32.into()))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Steam ID64"))
								.schema_type(SchemaType::Integer)
								.example(Some(76561198282622073_u64.into()))
								.build(),
						))
						.build(),
				)
				.into(),
			)
		}
	}

	impl IntoParams for SteamID {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("steam_id")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
