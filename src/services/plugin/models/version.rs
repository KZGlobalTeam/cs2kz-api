//! Custom wrapper around [`semver::Version`] so we can override trait
//! implementations.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A CS2KZ plugin version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, utoipa::ToSchema)]
#[schema(value_type = str, example = "0.0.1")]
pub struct PluginVersion(pub semver::Version);

impl fmt::Display for PluginVersion
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(&self.0, f)
	}
}

impl Serialize for PluginVersion
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for PluginVersion
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut s = &*String::deserialize(deserializer)?;

		if s.starts_with('v') {
			s = &s[1..];
		}

		s.parse::<semver::Version>()
			.map(Self)
			.map_err(serde::de::Error::custom)
	}
}

crate::macros::sqlx_scalar_forward!(PluginVersion as String => {
	encode: |self| { self.to_string() },
	decode: |value| { value.parse::<semver::Version>().map(Self)? },
});
