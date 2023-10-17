// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::SteamID,
	std::{borrow::Cow, fmt::Display},
	utoipa::ToSchema,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, ToSchema)]
pub enum PlayerIdentifier<'a> {
	SteamID(SteamID),
	Name(Cow<'a, str>),
}

impl<'a> PlayerIdentifier<'a> {
	#[inline]
	pub fn name<S>(name: S) -> Self
	where
		S: Into<Cow<'a, str>>, {
		Self::Name(name.into())
	}
}

impl From<SteamID> for PlayerIdentifier<'_> {
	fn from(steam_id: SteamID) -> Self {
		Self::SteamID(steam_id)
	}
}

impl<'a, T> From<T> for PlayerIdentifier<'a>
where
	T: Into<Cow<'a, str>>,
{
	fn from(value: T) -> Self {
		Self::Name(value.into())
	}
}

impl Display for PlayerIdentifier<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PlayerIdentifier::SteamID(steam_id) => write!(f, "{steam_id}"),
			PlayerIdentifier::Name(name) => write!(f, "{name}"),
		}
	}
}

mod serde_impls {
	use {
		super::{PlayerIdentifier, SteamID},
		serde::{Deserialize, Deserializer, Serialize, Serializer},
		std::borrow::Cow,
	};

	impl Serialize for PlayerIdentifier<'_> {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer, {
			match self {
				PlayerIdentifier::SteamID(steam_id) => steam_id.serialize(serializer),
				PlayerIdentifier::Name(name) => name.serialize(serializer),
			}
		}
	}

	#[derive(Deserialize)]
	#[serde(untagged)]
	enum Deserializable<'a> {
		SteamID(SteamID),
		Name(&'a str),
	}

	impl<'de> Deserialize<'de> for PlayerIdentifier<'de> {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>, {
			match Deserializable::deserialize(deserializer)? {
				Deserializable::SteamID(steam_id) => Ok(Self::SteamID(steam_id)),
				Deserializable::Name(name) => Ok(Self::Name(Cow::Borrowed(name))),
			}
		}
	}
}
