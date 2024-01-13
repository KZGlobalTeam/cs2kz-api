use std::str::FromStr;

use derive_more::Display;

use crate::{Error, Result};

#[repr(u8)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Jumpstat {
	#[cfg_attr(feature = "utoipa", schema(rename = "longjump"))]
	LongJump = 1,

	#[cfg_attr(feature = "utoipa", schema(rename = "single_bhop"))]
	SingleBhop = 2,

	#[cfg_attr(feature = "utoipa", schema(rename = "multi_bhop"))]
	MultiBhop = 3,

	#[cfg_attr(feature = "utoipa", schema(rename = "weirdjump"))]
	WeirdJump = 4,

	#[cfg_attr(feature = "utoipa", schema(rename = "ladderjump"))]
	LadderJump = 5,

	#[cfg_attr(feature = "utoipa", schema(rename = "ladderhop"))]
	LadderHop = 6,
}

impl Jumpstat {
	/// Formats the jumpstat in a standardized way that is consistent with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::LongJump => "longjump",
			Self::SingleBhop => "single_bhop",
			Self::MultiBhop => "multi_bhop",
			Self::WeirdJump => "weirdjump",
			Self::LadderJump => "ladderjump",
			Self::LadderHop => "ladderhop",
		}
	}

	/// Formats the jumpstat as an abbreviation.
	#[inline]
	pub const fn short(&self) -> &'static str {
		match self {
			Self::LongJump => "LJ",
			Self::SingleBhop => "BH",
			Self::MultiBhop => "MBH",
			Self::WeirdJump => "WJ",
			Self::LadderJump => "LAJ",
			Self::LadderHop => "LAH",
		}
	}
}

impl From<Jumpstat> for u8 {
	#[inline]
	fn from(value: Jumpstat) -> Self {
		value as u8
	}
}

impl TryFrom<u8> for Jumpstat {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::LongJump),
			2 => Ok(Self::SingleBhop),
			3 => Ok(Self::MultiBhop),
			4 => Ok(Self::WeirdJump),
			5 => Ok(Self::LadderJump),
			6 => Ok(Self::LadderHop),
			_ => Err(Error::InvalidJumpstatID { value }),
		}
	}
}

impl FromStr for Jumpstat {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if value.eq_ignore_ascii_case("longjump") {
			return Ok(Self::LongJump);
		}

		if value.eq_ignore_ascii_case("single_bhop") {
			return Ok(Self::SingleBhop);
		}

		if value.eq_ignore_ascii_case("multi_bhop") {
			return Ok(Self::MultiBhop);
		}

		if value.eq_ignore_ascii_case("weirdjump") {
			return Ok(Self::WeirdJump);
		}

		if value.eq_ignore_ascii_case("ladderjump") {
			return Ok(Self::LadderJump);
		}

		if value.eq_ignore_ascii_case("ladderhop") {
			return Ok(Self::LadderHop);
		}

		Err(Error::InvalidJumpstat { value: value.to_owned() })
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Jumpstat;
	use crate::serde::IntOrStr;

	impl Jumpstat {
		/// Serializes the given `jumpstat` in the standardized API format.
		pub fn serialize_api<S: Serializer>(
			jumpstat: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			jumpstat.api().serialize(serializer)
		}

		/// Serializes the given `jumpstat` as an ID.
		pub fn serialize_id<S: Serializer>(
			jumpstat: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			(*jumpstat as u8).serialize(serializer)
		}
	}

	impl Serialize for Jumpstat {
		/// By default [`Jumpstat::serialize_api`] is used for serialization, but you can use
		/// any of the `serialize_*` functions and pass them to
		/// `#[serde(serialize_with = "...")]` if you need a different method.
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			Self::serialize_api(self, serializer)
		}
	}

	impl Jumpstat {
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

	impl<'de> Deserialize<'de> for Jumpstat {
		/// The default [`Deserialize`] implementation is a best-effort.
		///
		/// This means it considers as many cases as possible; if you want / need
		/// a specific format, consider using `#[serde(deserialize_with = "...")]` in
		/// combination with any of the `deserialize_*` methods on [`Jumpstat`].
		fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			match <IntOrStr<u8> as Deserialize<'de>>::deserialize(deserializer)? {
				IntOrStr::Int(value) => value.try_into(),
				IntOrStr::Str(value) => value.parse(),
			}
			.map_err(serde::de::Error::custom)
		}
	}
}

#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use super::Jumpstat;

	crate::sqlx::from_row_as!(Jumpstat as u8 {
		encode: |jumpstat| { *jumpstat as u8 }
		decode: |int| { Jumpstat::try_from(int) }
	});
}

#[cfg(feature = "utoipa")]
crate::utoipa::into_params!(Jumpstat as "jumpstat": "");
