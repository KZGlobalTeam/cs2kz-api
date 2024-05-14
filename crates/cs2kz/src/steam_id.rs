//! A type for working with [Valve's SteamIDs].
//!
//! [Valve's SteamIDs]: https://developer.valvesoftware.com/wiki/SteamID

use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};
use std::num::{NonZeroU32, NonZeroU64, ParseIntError};
use std::ops::Deref;
use std::str::FromStr;
use std::{cmp, mem, ptr};

use thiserror::Error;

/// Static assertion to ensure `SteamID` is null-pointer-optimized.
const _ASSERT_NPO: () = assert! {
	mem::size_of::<SteamID>() == mem::size_of::<Option<SteamID>>(),
	"`SteamID` has an unexpected ABI"
};

/// A type for working with [Valve's SteamIDs].
///
/// This type is a thin wrapper around a 64-bit integer, as described by Valve. It has a bunch of
/// useful methods and trait implementations that can be used to encode, decode, and format
/// SteamIDs.
///
/// [Valve's SteamIDs]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID {
	/// The SteamID.
	value: NonZeroU64,
}

impl SteamID {
	/// The minimum value for a valid [`SteamID`].
	pub const MIN: Self = unsafe { Self::new_unchecked(76561197960265729_u64) };

	/// The maximum value for a valid [`SteamID`].
	pub const MAX: Self = unsafe { Self::new_unchecked(76561202255233023_u64) };

	/// Used for bit manipulation.
	const MAGIC_OFFSET: u64 = Self::MIN.value.get() - 1;

	/// Create a new [`SteamID`], without checking that `value` is in-range.
	///
	/// # Safety
	///
	/// The caller must ensure that `value` is within `SteamID::MIN..=SteamID::MAX`.
	pub const unsafe fn new_unchecked(value: u64) -> Self {
		debug_assert! {
			76561197960265729_u64 <= value && value <= 76561202255233023_u64,
			"SteamID out of bounds"
		};

		Self {
			// SAFETY: the caller must ensure that `value` is in a valid range
			value: unsafe { NonZeroU64::new_unchecked(value) },
		}
	}

	/// Returns the underlying 64-bit integer.
	pub const fn value(&self) -> NonZeroU64 {
		self.value
	}

	/// Returns the `X` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn x(&self) -> u64 {
		let x = self.value.get() >> 56;

		debug_assert!(matches!(x, 0 | 1), "SteamID X segment has an invalid value");

		x
	}

	/// Returns the `Y` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn y(&self) -> u64 {
		let y = self.value.get() & 1;

		debug_assert!(matches!(y, 0 | 1), "SteamID Y segment has an invalid value");

		y
	}

	/// Returns the `Z` segment in `STEAM_X:Y:Z`.
	pub const fn z(&self) -> u64 {
		(self.value.get() - Self::MAGIC_OFFSET - self.y()) / 2
	}

	/// Returns the `SteamID` in its 64-bit representation.
	pub const fn as_u64(&self) -> u64 {
		self.value.get()
	}

	/// Returns the `SteamID` in its 32-bit representation.
	#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
	pub const fn as_u32(&self) -> u32 {
		let value = ((self.z() + self.y()) * 2) - self.y();

		debug_assert!(
			0 < value && value <= (u32::MAX as u64),
			"SteamID 32-bit representation has an invalid value"
		);

		value as u32
	}

	/// Returns the `SteamID` in its "Steam3ID" representation.
	pub fn as_id3(&self) -> String {
		format!("U:1:{}", self.as_u32())
	}

	/// Returns a `SteamID`, if the given `value` is in-range.
	pub const fn from_u64(value: u64) -> Option<Self> {
		if Self::MIN.value.get() <= value && value <= Self::MAX.value.get() {
			// SAFETY: we checked that `value` is in-range
			Some(unsafe { Self::new_unchecked(value) })
		} else {
			None
		}
	}

	/// Returns a `SteamID`, if the given `value` is in-range.
	#[allow(clippy::as_conversions)]
	pub const fn from_u32(value: u32) -> Option<Self> {
		Self::from_u64((value as u64) + Self::MAGIC_OFFSET)
	}

	/// Parses a [`SteamID`] in the standard format of `STEAM_X:Y:Z`.
	pub fn from_standard<S>(value: S) -> Result<Self, ParseSteamIDError>
	where
		S: AsRef<str>,
	{
		let mut value = value.as_ref();

		if value.starts_with("STEAM_") {
			value = value.trim_start_matches("STEAM_");
		} else {
			return Err(ParseSteamIDError::MissingPrefix);
		}

		let mut segments = value.split(':');

		match segments.next() {
			Some("0" | "1") => {}
			Some(_) => {
				return Err(ParseSteamIDError::InvalidX);
			}
			None => {
				return Err(ParseSteamIDError::MissingX);
			}
		}

		let y = segments
			.next()
			.ok_or(ParseSteamIDError::MissingY)?
			.parse::<u64>()
			.map_err(|err| ParseSteamIDError::InvalidY(Some(err)))?;

		if y > 1 {
			return Err(ParseSteamIDError::InvalidY(None));
		}

		let z = segments
			.next()
			.ok_or(ParseSteamIDError::MissingZ)?
			.parse::<u64>()
			.map_err(ParseSteamIDError::InvalidZ)?;

		if y == 0 && z == 0 {
			return Err(ParseSteamIDError::IsZero);
		}

		if (z + Self::MAGIC_OFFSET) > Self::MAX {
			return Err(ParseSteamIDError::OutOfRange);
		}

		Self::from_u64(Self::MAGIC_OFFSET | y | (z << 1)).ok_or(ParseSteamIDError::OutOfRange)
	}

	/// Parses a "Steam3ID" into a [`SteamID`].
	///
	/// The expected input format is `U:1:322356345`, optionally enclosed in `[]`.
	pub fn from_id3<S>(value: S) -> Result<Self, ParseSteam3IDError>
	where
		S: AsRef<str>,
	{
		let mut value = value.as_ref();

		match (value.starts_with('['), value.ends_with(']')) {
			(false, false) => {}
			(true, true) => {
				value = value.trim_start_matches('[').trim_end_matches(']');
			}
			(true, false) | (false, true) => {
				return Err(ParseSteam3IDError::InconsistentBrackets);
			}
		}

		let mut segments = value.split(':');

		let Some("U") = segments.next() else {
			return Err(ParseSteam3IDError::MissingAccountType);
		};

		let Some("1") = segments.next() else {
			return Err(ParseSteam3IDError::MissingOne);
		};

		let id = segments
			.next()
			.ok_or(ParseSteam3IDError::MissingID)?
			.parse::<u32>()
			.map_err(ParseSteam3IDError::InvalidID)?;

		Self::from_u32(id).ok_or(ParseSteam3IDError::OutOfRange)
	}
}

/// Potential errors that can occur when parsing a Steam3ID.
#[derive(Debug, Clone, Error)]
pub enum ParseSteamIDError {
	/// Every SteamID starts with `STEAM_`.
	#[error("missing `STEAM_` prefix")]
	MissingPrefix,

	/// The SteamID ended after the `STEAM_` prefix.
	#[error("missing `X` segment")]
	MissingX,

	/// The `X` segment was something other than 0 or 1.
	#[error("invalid `X` segment; expected 0 or 1")]
	InvalidX,

	/// The SteamID was missing the `Y` segment.
	#[error("missing `Y` segment")]
	MissingY,

	/// The `Y` segment was something other than 0 or 1.
	#[error("invalid `Y` segment; expected 0 or 1{}", match .0 {
		None => String::new(),
		Some(err) => format!(" ({err})"),
	})]
	InvalidY(Option<ParseIntError>),

	/// The SteamID was missing the `Z` segment.
	#[error("missing `Z` segment")]
	MissingZ,

	/// The `Z` segment was not a valid `u64`.
	#[error("invalid `Z` segment: {0}")]
	InvalidZ(ParseIntError),

	/// SteamIDs can't have a value of 0.
	#[error("is zero")]
	IsZero,

	/// SteamID value was otherwise out of bounds.
	#[error("64-bit SteamID out of range")]
	OutOfRange,
}

/// Potential errors that can occur when parsing a Steam3ID.
#[derive(Debug, Clone, Error)]
pub enum ParseSteam3IDError {
	/// Steam3IDs can optionally be enclosed by `[]`, e.g., `[U:1:322356345]`.
	///
	/// If such a string is passed, but it has exactly one of either the opening or closing
	/// bracket, that's a malformed ID.
	#[error("got one of, but not both, opening or closing bracket")]
	InconsistentBrackets,

	/// Steam3IDs contain an account type as their first segment.
	///
	/// For CS2 this is typically `U`.
	#[error("missing `U` segment specifying account type")]
	MissingAccountType,

	/// Steam3IDs _always_ have a `1` as their second segment.
	#[error("missing `1` segment")]
	MissingOne,

	/// Steam3IDs have the 32-bit ID representation as their third segment.
	#[error("missing ID segment")]
	MissingID,

	/// Parsing the ID segment failed, because it was not a valid `u32`.
	#[error("invalid 32-bit SteamID: {0}")]
	InvalidID(ParseIntError),

	/// Parsing the ID segment failed, because the value was out of range for a legal SteamID.
	#[error("32-bit SteamID out of range")]
	OutOfRange,
}

impl Display for SteamID {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
	}
}

impl fmt::Binary for SteamID {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Binary::fmt(&self.value, f)
	}
}

impl fmt::LowerHex for SteamID {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::LowerHex::fmt(&self.value, f)
	}
}

impl fmt::UpperHex for SteamID {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::UpperHex::fmt(&self.value, f)
	}
}

impl fmt::Octal for SteamID {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Octal::fmt(&self.value, f)
	}
}

impl Borrow<NonZeroU64> for SteamID {
	fn borrow(&self) -> &NonZeroU64 {
		&self.value
	}
}

impl Borrow<u64> for SteamID {
	fn borrow(&self) -> &u64 {
		// SAFETY: `NonZeroU64` has the same ABI as `u64`.
		unsafe { &*ptr::from_ref(<Self as Borrow<NonZeroU64>>::borrow(self)).cast() }
	}
}

impl AsRef<NonZeroU64> for SteamID {
	fn as_ref(&self) -> &NonZeroU64 {
		self.borrow()
	}
}

impl AsRef<u64> for SteamID {
	fn as_ref(&self) -> &u64 {
		self.borrow()
	}
}

impl Deref for SteamID {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		self.borrow()
	}
}

impl PartialEq<NonZeroU64> for SteamID {
	fn eq(&self, other: &NonZeroU64) -> bool {
		PartialEq::eq(&self.value, other)
	}
}

impl PartialEq<SteamID> for NonZeroU64 {
	fn eq(&self, other: &SteamID) -> bool {
		PartialEq::eq(self, &other.value)
	}
}

impl PartialEq<u64> for SteamID {
	fn eq(&self, other: &u64) -> bool {
		PartialEq::<u64>::eq(other, self.borrow())
	}
}

impl PartialEq<SteamID> for u64 {
	fn eq(&self, other: &SteamID) -> bool {
		PartialEq::<u64>::eq(self, other.borrow())
	}
}

impl PartialEq<NonZeroU32> for SteamID {
	fn eq(&self, other: &NonZeroU32) -> bool {
		PartialEq::eq(&self.as_u32(), &other.get())
	}
}

impl PartialEq<SteamID> for NonZeroU32 {
	fn eq(&self, other: &SteamID) -> bool {
		PartialEq::eq(&self.get(), &other.as_u32())
	}
}

impl PartialEq<u32> for SteamID {
	fn eq(&self, other: &u32) -> bool {
		PartialEq::eq(&self.as_u32(), other)
	}
}

impl PartialEq<SteamID> for u32 {
	fn eq(&self, other: &SteamID) -> bool {
		PartialEq::eq(self, &other.as_u32())
	}
}

impl PartialOrd<NonZeroU64> for SteamID {
	fn partial_cmp(&self, other: &NonZeroU64) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(&self.value, other)
	}
}

impl PartialOrd<SteamID> for NonZeroU64 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(self, &other.value)
	}
}

impl PartialOrd<u64> for SteamID {
	fn partial_cmp(&self, other: &u64) -> Option<cmp::Ordering> {
		PartialOrd::<u64>::partial_cmp(other, self.borrow())
	}
}

impl PartialOrd<SteamID> for u64 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		PartialOrd::<u64>::partial_cmp(self, other.borrow())
	}
}

impl PartialOrd<NonZeroU32> for SteamID {
	fn partial_cmp(&self, other: &NonZeroU32) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(&self.as_u32(), &other.get())
	}
}

impl PartialOrd<SteamID> for NonZeroU32 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(&self.get(), &other.as_u32())
	}
}

impl PartialOrd<u32> for SteamID {
	fn partial_cmp(&self, other: &u32) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(&self.as_u32(), other)
	}
}

impl PartialOrd<SteamID> for u32 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		PartialOrd::partial_cmp(self, &other.as_u32())
	}
}

impl From<SteamID> for NonZeroU64 {
	fn from(value: SteamID) -> Self {
		value.value()
	}
}

impl From<SteamID> for u64 {
	fn from(value: SteamID) -> Self {
		value.as_u64()
	}
}

impl From<SteamID> for NonZeroU32 {
	fn from(value: SteamID) -> Self {
		// SAFETY: if `SteamID::value` is non-zero, then `SteamID::as_u32()`
		//         should also be non-zero
		unsafe { NonZeroU32::new_unchecked(value.as_u32()) }
	}
}

impl From<SteamID> for u32 {
	fn from(value: SteamID) -> Self {
		value.as_u32()
	}
}

/// Converting to a [`SteamID`] failed, because the value was out of range for a valid `SteamID`.
#[derive(Debug, Clone, Copy, Error)]
#[error("value was out of range for a valid SteamID")]
pub struct SteamIDOutOfRange;

impl TryFrom<NonZeroU64> for SteamID {
	type Error = SteamIDOutOfRange;

	fn try_from(value: NonZeroU64) -> Result<Self, Self::Error> {
		Self::try_from(value.get())
	}
}

impl TryFrom<u64> for SteamID {
	type Error = SteamIDOutOfRange;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		if let Ok(value) = u32::try_from(value) {
			Self::try_from(value)
		} else {
			Self::from_u64(value).ok_or(SteamIDOutOfRange)
		}
	}
}

impl TryFrom<NonZeroU32> for SteamID {
	type Error = SteamIDOutOfRange;

	fn try_from(value: NonZeroU32) -> Result<Self, Self::Error> {
		Self::try_from(value.get())
	}
}

impl TryFrom<u32> for SteamID {
	type Error = SteamIDOutOfRange;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		Self::from_u32(value).ok_or(SteamIDOutOfRange)
	}
}

/// Parsing a [`SteamID`] from a string failed.
#[derive(Debug, Clone, Copy, Error)]
pub enum InvalidSteamID {
	/// String could be parsed into an integer, but the integer was invalid.
	#[error(transparent)]
	InvalidU64(#[from] SteamIDOutOfRange),

	/// String could not be parsed as any known format.
	#[error("failed to parse SteamID (unrecognized format)")]
	UnrecognizedSteamIDFormat,
}

impl FromStr for SteamID {
	type Err = InvalidSteamID;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if let Ok(int) = s.parse::<u64>() {
			return Self::try_from(int).map_err(Into::into);
		}

		if let Ok(steam_id) = Self::from_standard(s) {
			return Ok(steam_id);
		}

		if let Ok(steam_id) = Self::from_id3(s) {
			return Ok(steam_id);
		}

		Err(InvalidSteamID::UnrecognizedSteamIDFormat)
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls {
	use std::str::FromStr;

	use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

	use super::SteamID;

	impl SteamID {
		/// Serialize in the standard `STEAM_X:Y:Z` format.
		pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.to_string().serialize(serializer)
		}

		/// Serialize in the "Steam3ID" format.
		pub fn serialize_id3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_id3().serialize(serializer)
		}

		/// Serialize as a 64-bit integer.
		pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_u64().serialize(serializer)
		}

		/// Serialize as a stringified 64-bit integer.
		pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_u64().to_string().serialize(serializer)
		}

		/// Serialize as a 32-bit integer.
		pub fn serialize_u32<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_u32().serialize(serializer)
		}
	}

	impl Serialize for SteamID {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.serialize_standard(serializer)
		}
	}

	impl SteamID {
		/// Deserialize as the standard `STEAM_X:Y:Z` format.
		pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			<&'de str>::deserialize(deserializer)
				.map(Self::from_standard)?
				.map_err(de::Error::custom)
		}

		/// Deserialize as the "Steam3ID" format.
		pub fn deserialize_id3<'de, D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			<&'de str>::deserialize(deserializer)
				.map(Self::from_id3)?
				.map_err(de::Error::custom)
		}

		/// Deserialize as a 64-bit integer.
		pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			u64::deserialize(deserializer)
				.map(Self::try_from)?
				.map_err(de::Error::custom)
		}

		/// Deserialize as a stringified 64-bit integer.
		pub fn deserialize_u64_stringified<'de, D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			<&'de str>::deserialize(deserializer)
				.map(<u64 as FromStr>::from_str)?
				.map_err(de::Error::custom)
				.map(Self::try_from)?
				.map_err(de::Error::custom)
		}

		/// Deserialize as a 32-bit integer.
		pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			u32::deserialize(deserializer)
				.map(Self::try_from)?
				.map_err(de::Error::custom)
		}
	}

	impl<'de> Deserialize<'de> for SteamID {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			#[derive(Deserialize)]
			#[serde(untagged)]
			#[allow(clippy::missing_docs_in_private_items)]
			enum Helper {
				U32(u32),
				U64(u64),
				Str(String),
			}

			Helper::deserialize(deserializer)
				.map(|value| match value {
					Helper::U32(int) => Self::try_from(int).map_err(Into::into),
					Helper::U64(int) => Self::try_from(int).map_err(Into::into),
					Helper::Str(str) => str.parse(),
				})?
				.map_err(de::Error::custom)
		}
	}
}

/// Method and Trait implementations when depending on [`sqlx`].
#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use std::borrow::Borrow;

	use sqlx::database::{HasArguments, HasValueRef};
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use super::SteamID;

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
			<u64 as Encode<'q, DB>>::encode_by_ref(self.borrow(), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for SteamID
	where
		DB: Database,
		u64: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			<u64 as Decode<'r, DB>>::decode(value)
				.map(Self::try_from)?
				.map_err(Into::into)
		}
	}
}

/// Method and Trait implementations when depending on [`utoipa`].
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
