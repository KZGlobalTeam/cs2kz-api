//! Utility type for identifying maps.

use std::fmt::{self, Display};

/// Maps are either identified by their ID, which is unique, or their name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum MapIdentifier {
	/// A map's ID.
	ID(u16),

	/// A map's name.
	Name(String),
}

impl Display for MapIdentifier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ID(id) => write!(f, "{id}"),
			Self::Name(name) => write!(f, "{name}"),
		}
	}
}

impl From<u16> for MapIdentifier {
	fn from(value: u16) -> Self {
		Self::ID(value)
	}
}

impl From<&str> for MapIdentifier {
	fn from(value: &str) -> Self {
		value
			.parse::<u16>()
			.map(Self::ID)
			.unwrap_or_else(|_| Self::Name(value.to_owned()))
	}
}

impl From<String> for MapIdentifier {
	fn from(value: String) -> Self {
		value
			.parse::<u16>()
			.map(Self::ID)
			.unwrap_or(Self::Name(value))
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod de {
		use serde::{Deserialize, Deserializer};

		use crate::MapIdentifier;

		impl<'de> Deserialize<'de> for MapIdentifier {
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Deserialize)]
				#[serde(untagged)]
				enum Helper {
					U16(u16),
					String(String),
				}

				Helper::deserialize(deserializer).map(|value| match value {
					Helper::U16(id) => Self::ID(id),
					Helper::String(id_or_name) => id_or_name
						.parse::<u16>()
						.map(Self::ID)
						.unwrap_or(Self::Name(id_or_name)),
				})
			}
		}
	}
}

#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::{IntoParams, ToSchema};

	use crate::MapIdentifier;

	impl IntoParams for MapIdentifier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("map")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
