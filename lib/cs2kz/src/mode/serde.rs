//! Trait implementations for the [`serde`] crate.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::Mode;

impl Mode
{
	/// Serializes a [`Mode`] as an integer.
	pub fn serialize_u8<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		u8::from(*self).serialize(serializer)
	}

	/// Serializes a [`Mode`] using [`Mode::as_str()`].
	pub fn serialize_str<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_str().serialize(serializer)
	}

	/// Serializes a [`Mode`] using [`Mode::as_str_capitalized()`].
	pub fn serialize_str_capitalized<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_str_capitalized().serialize(serializer)
	}

	/// Serializes a [`Mode`] using [`Mode::as_str_short()`].
	pub fn serialize_str_short<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_str_short().serialize(serializer)
	}
}

impl Serialize for Mode
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_str(serializer)
	}
}

impl<'de> Deserialize<'de> for Mode
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
			U8(u8),
			Str(String),
		}

		Helper::deserialize(deserializer).and_then(|value| match value {
			Helper::U8(int) => Self::try_from(int).map_err(de::Error::custom),
			Helper::Str(str) => str.parse().map_err(de::Error::custom),
		})
	}
}
