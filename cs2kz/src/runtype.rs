use std::fmt::Display;
use std::str::FromStr;

use crate::{Error, Result};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(rename_all = "lowercase"))]
pub enum Runtype {
	#[default]
	Pro = 0,
	TP = 1,
}

impl Runtype {
	pub const fn api(&self) -> &'static str {
		match self {
			Runtype::Pro => "pro",
			Runtype::TP => "tp",
		}
	}
}

impl Display for Runtype {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{self:?}")
	}
}

impl From<bool> for Runtype {
	fn from(has_teleports: bool) -> Self {
		match has_teleports {
			false => Self::Pro,
			true => Self::TP,
		}
	}
}

impl From<Runtype> for bool {
	fn from(runtype: Runtype) -> Self {
		runtype == Runtype::TP
	}
}

impl TryFrom<&str> for Runtype {
	type Error = Error;

	fn try_from(input: &str) -> Result<Self> {
		input.parse()
	}
}

impl TryFrom<String> for Runtype {
	type Error = Error;

	fn try_from(input: String) -> Result<Self> {
		Self::try_from(input.as_str())
	}
}

impl FromStr for Runtype {
	type Err = Error;

	fn from_str(input: &str) -> Result<Self> {
		match input {
			"pro" | "false" => Ok(Self::Pro),
			"tp" | "true" => Ok(Self::TP),
			_ => Err(Error::InvalidRuntype { input: input.to_owned(), reason: None }),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	use super::Runtype;

	impl Runtype {
		pub fn serialize_as_text<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.api().serialize(serializer)
		}

		pub fn serialize_as_bool<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			bool::from(*self).serialize(serializer)
		}
	}

	impl Serialize for Runtype {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.serialize_as_text(serializer)
		}
	}

	#[derive(Deserialize)]
	#[serde(untagged)]
	enum Deserializable {
		Bool(bool),
		String(String),
	}

	impl<'de> Deserialize<'de> for Runtype {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			match Deserializable::deserialize(deserializer)? {
				Deserializable::Bool(bool) => Ok(bool.into()),
				Deserializable::String(input) => input.parse(),
			}
			.map_err(serde::de::Error::custom)
		}
	}
}
