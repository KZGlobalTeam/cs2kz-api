use std::str::FromStr;

use derive_more::Display;

use crate::{Error, Result};

#[repr(u8)]
#[derive(Default, Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Mode {
	#[default]
	#[display("VNL")]
	#[cfg_attr(feature = "utoipa", schema(rename = "kz_vanilla"))]
	Vanilla = 1,

	#[display("MKZ")]
	#[cfg_attr(feature = "utoipa", schema(rename = "kz_modded"))]
	Modded = 2,
}

impl Mode {
	/// Formats the mode in a standardized way that is consistent with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::Vanilla => "kz_vanilla",
			Self::Modded => "kz_modded",
		}
	}

	/// Formats the mode as an abbreviation.
	#[inline]
	pub const fn short(&self) -> &'static str {
		match self {
			Self::Vanilla => "VNL",
			Self::Modded => "MKZ",
		}
	}
}

impl From<Mode> for u8 {
	#[inline]
	fn from(value: Mode) -> Self {
		value as u8
	}
}

impl TryFrom<u8> for Mode {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::Vanilla),
			2 => Ok(Self::Modded),
			_ => Err(Error::InvalidModeID { value }),
		}
	}
}

impl FromStr for Mode {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if value.eq_ignore_ascii_case("kz_vanilla") || value.eq_ignore_ascii_case("vnl") {
			return Ok(Self::Vanilla);
		}

		if value.eq_ignore_ascii_case("kz_modded") || value.eq_ignore_ascii_case("mkz") {
			return Ok(Self::Modded);
		}

		Err(Error::InvalidMode { value: value.to_owned() })
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Mode;

	impl Mode {
		/// Serializes the given `mode` in the standardized API format.
		pub fn serialize_api<S: Serializer>(mode: &Self, serializer: S) -> Result<S::Ok, S::Error> {
			mode.api().serialize(serializer)
		}

		/// Serializes the given `mode` as an abbreviation.
		pub fn serialize_short<S: Serializer>(
			mode: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			mode.short().serialize(serializer)
		}

		/// Serializes the given `mode` as an integer.
		pub fn serialize_integer<S: Serializer>(
			mode: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			(*mode as u8).serialize(serializer)
		}
	}

	impl Serialize for Mode {
		/// By default [`Mode::serialize_api`] is used for serialization, but you can use
		/// any of the `serialize_*` functions and pass them to
		/// `#[serde(serialize_with = "...")]` if you need a different method.
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			Self::serialize_api(self, serializer)
		}
	}

	impl Mode {
		/// Deserializes from a string.
		pub fn deserialize_str<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			<&str as Deserialize>::deserialize(deserializer)?
				.parse()
				.map_err(serde::de::Error::custom)
		}

		/// Deserializes from an integer.
		pub fn deserialize_integer<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> Result<Self, D::Error> {
			<u8>::deserialize(deserializer)?
				.try_into()
				.map_err(serde::de::Error::custom)
		}
	}

	impl<'de> Deserialize<'de> for Mode {
		/// The default [`Deserialize`] implementation is a best-effort.
		///
		/// This means it considers as many cases as possible; if you want / need
		/// a specific format, consider using `#[serde(deserialize_with = "...")]` in
		/// combination with any of the `deserialize_*` methods on [`Mode`].
		fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			#[derive(Deserialize)]
			#[serde(untagged)]
			enum Helper<'a> {
				U8(u8),
				Str(&'a str),
			}

			match <Helper as Deserialize<'de>>::deserialize(deserializer)? {
				Helper::U8(value) => value.try_into(),
				Helper::Str(value) => value.parse(),
			}
			.map_err(serde::de::Error::custom)
		}
	}
}

#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use sqlx::database::{HasArguments, HasValueRef};
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use super::Mode;

	impl<DB: Database> Type<DB> for Mode
	where
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'row, DB: Database> Decode<'row, DB> for Mode
	where
		u8: Decode<'row, DB>,
	{
		fn decode(value: <DB as HasValueRef<'row>>::ValueRef) -> Result<Self, BoxDynError> {
			Self::try_from(<u8 as Decode<'row, DB>>::decode(value)?).map_err(Into::into)
		}
	}

	impl<'query, DB: Database> Encode<'query, DB> for Mode
	where
		u8: Encode<'query, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'query>>::ArgumentBuffer) -> IsNull {
			<u8 as Encode<'query, DB>>::encode(*self as u8, buf)
		}
	}
}
