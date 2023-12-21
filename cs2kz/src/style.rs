use std::str::FromStr;

use derive_more::Display;

use crate::{Error, Result};

#[repr(u8)]
#[derive(Default, Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(rename_all = "lowercase"))]
pub enum Style {
	#[default]
	Normal = 1,
	Backwards = 2,
	Sideways = 3,
	WOnly = 4,
}

impl Style {
	/// Formats the style in a standardized way that is consistent with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::Normal => "normal",
			Self::Backwards => "backwards",
			Self::Sideways => "sideways",
			Self::WOnly => "w_only",
		}
	}
}

impl From<Style> for u8 {
	#[inline]
	fn from(value: Style) -> Self {
		value as u8
	}
}

impl TryFrom<u8> for Style {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::Normal),
			2 => Ok(Self::Backwards),
			3 => Ok(Self::Sideways),
			4 => Ok(Self::WOnly),
			_ => Err(Error::InvalidStyleID { value }),
		}
	}
}

impl FromStr for Style {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if value.eq_ignore_ascii_case("normal") {
			return Ok(Self::Normal);
		}

		if value.eq_ignore_ascii_case("backwards") {
			return Ok(Self::Backwards);
		}

		if value.eq_ignore_ascii_case("sideways") {
			return Ok(Self::Sideways);
		}

		if value.eq_ignore_ascii_case("w_only") || value.eq_ignore_ascii_case("w-only") {
			return Ok(Self::WOnly);
		}

		Err(Error::InvalidStyle { value: value.to_owned() })
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Style;
	use crate::serde::IntOrStr;

	impl Style {
		/// Serializes the given `style` in the standardized API format.
		pub fn serialize_api<S: Serializer>(
			style: &Self,
			serializer: S,
		) -> Result<S::Ok, S::Error> {
			style.api().serialize(serializer)
		}

		/// Serializes the given `style` as an ID.
		pub fn serialize_id<S: Serializer>(style: &Self, serializer: S) -> Result<S::Ok, S::Error> {
			(*style as u8).serialize(serializer)
		}
	}

	impl Serialize for Style {
		/// By default [`Style::serialize_api`] is used for serialization, but you can use
		/// any of the `serialize_*` functions and pass them to
		/// `#[serde(serialize_with = "...")]` if you need a different method.
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			Self::serialize_api(self, serializer)
		}
	}

	impl Style {
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

	impl<'de> Deserialize<'de> for Style {
		/// The default [`Deserialize`] implementation is a best-effort.
		///
		/// This means it considers as many cases as possible; if you want / need
		/// a specific format, consider using `#[serde(deserialize_with = "...")]` in
		/// combination with any of the `deserialize_*` methods on [`Style`].
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
	use super::Style;

	crate::sqlx::from_row_as!(Style as u8 {
		encode: |style| { *style as u8 }
		decode: |int| { Style::try_from(int) }
	});
}
