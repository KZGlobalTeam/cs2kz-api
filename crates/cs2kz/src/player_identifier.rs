use std::borrow::Cow;

use derive_more::Display;

use crate::SteamID;

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum PlayerIdentifier<'a> {
	SteamID(SteamID),
	Name(Cow<'a, str>),
}

impl From<SteamID> for PlayerIdentifier<'_> {
	#[inline]
	fn from(value: SteamID) -> Self {
		Self::SteamID(value)
	}
}

impl<'a, T: Into<Cow<'a, str>>> From<T> for PlayerIdentifier<'a> {
	fn from(value: T) -> Self {
		Self::Name(value.into())
	}
}

#[cfg(feature = "utoipa")]
crate::utoipa::into_params!(PlayerIdentifier<'_> as "player": "A player's SteamID or Name");
