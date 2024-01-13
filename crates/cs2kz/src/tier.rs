use std::str::FromStr;

use derive_more::Display;

use crate::{Error, Result};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(rename_all = "snake_case"))]
pub enum Tier {
	#[display("Very Easy")]
	VeryEasy = 1,
	Easy = 2,
	Medium = 3,
	Advanced = 4,
	Hard = 5,
	#[display("Very Hard")]
	VeryHard = 6,
	Extreme = 7,
	Death = 8,
	Unfeasible = 9,
	Impossible = 10,
}

impl Tier {
	/// Formats the tier in a standardized way that is consistent with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::VeryEasy => "very_easy",
			Self::Easy => "easy",
			Self::Medium => "medium",
			Self::Advanced => "advanced",
			Self::Hard => "hard",
			Self::VeryHard => "very_hard",
			Self::Extreme => "extreme",
			Self::Death => "death",
			Self::Unfeasible => "unfeasible",
			Self::Impossible => "impossible",
		}
	}
}

impl From<Tier> for u8 {
	#[inline]
	fn from(value: Tier) -> Self {
		value as u8
	}
}

impl TryFrom<u8> for Tier {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::VeryEasy),
			2 => Ok(Self::Easy),
			3 => Ok(Self::Medium),
			4 => Ok(Self::Advanced),
			5 => Ok(Self::Hard),
			6 => Ok(Self::VeryHard),
			7 => Ok(Self::Extreme),
			8 => Ok(Self::Death),
			9 => Ok(Self::Unfeasible),
			10 => Ok(Self::Impossible),
			_ => Err(Error::InvalidTier { value: value.to_string() }),
		}
	}
}

impl FromStr for Tier {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if value.eq_ignore_ascii_case("very_easy") {
			return Ok(Self::VeryEasy);
		}

		if value.eq_ignore_ascii_case("easy") {
			return Ok(Self::Easy);
		}

		if value.eq_ignore_ascii_case("medium") {
			return Ok(Self::Medium);
		}

		if value.eq_ignore_ascii_case("advanced") {
			return Ok(Self::Advanced);
		}

		if value.eq_ignore_ascii_case("hard") {
			return Ok(Self::Hard);
		}

		if value.eq_ignore_ascii_case("very_hard") {
			return Ok(Self::VeryHard);
		}

		if value.eq_ignore_ascii_case("extreme") {
			return Ok(Self::Extreme);
		}

		if value.eq_ignore_ascii_case("death") {
			return Ok(Self::Death);
		}

		if value.eq_ignore_ascii_case("unfeasible") {
			return Ok(Self::Unfeasible);
		}

		if value.eq_ignore_ascii_case("impossible") {
			return Ok(Self::Impossible);
		}

		Err(Error::InvalidTier { value: value.to_owned() })
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Tier;
	use crate::serde::IntOrStr;

	impl Tier {
		/// Serializes the given `tier` in the standardized API format.
		pub fn serialize_api<S: Serializer>(tier: &Self, serializer: S) -> Result<S::Ok, S::Error> {
			tier.api().serialize(serializer)
		}

		/// Serializes the given `tier` as an integer.
		pub fn serialize_integer<S: Serializer>(
			tier: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			(*tier as u8).serialize(serializer)
		}
	}

	impl Serialize for Tier {
		/// By default [`Tier::serialize_api`] is used for serialization, but you can use
		/// any of the `serialize_*` functions and pass them to
		/// `#[serde(serialize_with = "...")]` if you need a different method.
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			Self::serialize_api(self, serializer)
		}
	}

	impl Tier {
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

	impl<'de> Deserialize<'de> for Tier {
		/// The default [`Deserialize`] implementation is a best-effort.
		///
		/// This means it considers as many cases as possible; if you want / need
		/// a specific format, consider using `#[serde(deserialize_with = "...")]` in
		/// combination with any of the `deserialize_*` methods on [`Tier`].
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
	use super::Tier;

	crate::sqlx::from_row_as!(Tier as u8 {
		encode: |tier| { *tier as u8 }
		decode: |int| { Tier::try_from(int) }
	});
}

#[cfg(feature = "utoipa")]
crate::utoipa::into_params!(Tier as "tier": "");
