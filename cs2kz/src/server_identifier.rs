use std::borrow::Cow;
use std::convert::Infallible;
use std::str::FromStr;

use derive_more::Display;

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum ServerIdentifier<'a> {
	ID(u16),
	Name(Cow<'a, str>),
}

impl From<u16> for ServerIdentifier<'_> {
	#[inline]
	fn from(value: u16) -> Self {
		Self::ID(value)
	}
}

impl<'a> From<&'a str> for ServerIdentifier<'a> {
	fn from(value: &'a str) -> Self {
		Self::Name(value.into())
	}
}

impl From<String> for ServerIdentifier<'_> {
	fn from(value: String) -> Self {
		Self::Name(value.into())
	}
}

impl<'a> From<Cow<'a, str>> for ServerIdentifier<'a> {
	fn from(value: Cow<'a, str>) -> Self {
		Self::Name(value)
	}
}

impl FromStr for ServerIdentifier<'_> {
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		Ok(match value.parse::<u16>() {
			Ok(id) => Self::ID(id),

			// This is kind of unfortunate.
			Err(_) => Self::Name(Cow::Owned(value.to_owned())),
		})
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer};

	use super::ServerIdentifier;

	impl<'de> Deserialize<'de> for ServerIdentifier<'_> {
		fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
			#[derive(Deserialize)]
			#[serde(untagged)]
			enum Helper {
				U16(u16),
				Str(String),
			}

			Ok(match <Helper as Deserialize<'de>>::deserialize(deserializer)? {
				Helper::U16(value) => value.into(),
				Helper::Str(value) => value.parse().unwrap_or_else(|_| value.into()),
			})
		}
	}
}
