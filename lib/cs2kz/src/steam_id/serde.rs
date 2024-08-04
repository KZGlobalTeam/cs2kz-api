//! Trait implementations for the [`serde`] crate.

use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::SteamID;

impl SteamID
{
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

impl Serialize for SteamID
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_standard(serializer)
	}
}

impl SteamID
{
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

impl<'de> Deserialize<'de> for SteamID
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		#[allow(clippy::missing_docs_in_private_items)]
		enum Helper
		{
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
