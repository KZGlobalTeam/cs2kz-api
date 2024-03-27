//! Utility type for identifying players.

use std::fmt::{self, Display};

use crate::SteamID;

/// Players are usually identified by their [SteamID], which is unique, or their name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum PlayerIdentifier {
	/// A [SteamID].
	SteamID(SteamID),

	/// A player's name.
	Name(String),
}

impl Display for PlayerIdentifier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::SteamID(steam_id) => write!(f, "{steam_id}"),
			Self::Name(name) => write!(f, "{name}"),
		}
	}
}

impl From<SteamID> for PlayerIdentifier {
	fn from(value: SteamID) -> Self {
		Self::SteamID(value)
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod de {
		use serde::{Deserialize, Deserializer};

		use crate::{PlayerIdentifier, SteamID};

		impl<'de> Deserialize<'de> for PlayerIdentifier {
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Deserialize)]
				#[serde(untagged)]
				enum Helper {
					SteamID(SteamID),
					String(String),
				}

				Helper::deserialize(deserializer).map(|value| match value {
					Helper::SteamID(steam_id) => Self::SteamID(steam_id),
					Helper::String(steam_id_or_name) => steam_id_or_name
						.parse::<SteamID>()
						.map(Self::SteamID)
						.unwrap_or(Self::Name(steam_id_or_name)),
				})
			}
		}
	}
}

#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::{IntoParams, ToSchema};

	use crate::PlayerIdentifier;

	impl IntoParams for PlayerIdentifier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("player")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
