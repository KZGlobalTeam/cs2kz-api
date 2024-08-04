//! Trait implementations for the [`serde`] crate.

use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Permissions;

impl Serialize for Permissions
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(None)?;

		for value in self.iter_names() {
			serializer.serialize_element(value)?;
		}

		serializer.end()
	}
}

impl<'de> Deserialize<'de> for Permissions
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
			Int(u64),
			Word(String),
			Words(Vec<String>),
		}

		Helper::deserialize(deserializer).and_then(|value| match value {
			Helper::Int(flags) => Ok(Self::new(flags)),
			Helper::Word(word) => word.parse::<Self>().map_err(serde::de::Error::custom),
			Helper::Words(words) => Ok(words
				.into_iter()
				.flat_map(|word| word.parse::<Self>())
				.fold(Self::NONE, |acc, curr| (acc | curr))),
		})
	}
}
