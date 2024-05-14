//! Helpers for [`serde`].

/// [`serde`] helpers for [`String`].
pub mod string {
	use serde::{Deserialize, Deserializer};

	/// Deserializes a `String`, but treats empty arrays as `None`.
	pub fn deserialize_empty_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let Some(value) = Option::<String>::deserialize(deserializer)? else {
			return Ok(None);
		};

		if value.is_empty() {
			return Ok(None);
		}

		Ok(Some(value))
	}
}

/// [`serde`] helpers for [`Vec<T>`].
pub mod vec {
	use serde::{de, Deserialize, Deserializer};

	/// Deserialize a `Vec<T>` that is non-empty.
	pub fn deserialize_non_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
	where
		D: Deserializer<'de>,
		T: Deserialize<'de>,
	{
		let vec = Vec::<T>::deserialize(deserializer)?;

		if vec.is_empty() {
			return Err(de::Error::invalid_length(0, &"1 or more"));
		}

		Ok(vec)
	}

	/// Deserializes a `Vec<T>`, but treats empty arrays as `None`.
	pub fn deserialize_empty_as_none<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
	where
		D: Deserializer<'de>,
		T: Deserialize<'de>,
	{
		let Some(vec) = Option::<Vec<T>>::deserialize(deserializer)? else {
			return Ok(None);
		};

		if vec.is_empty() {
			return Ok(None);
		}

		Ok(Some(vec))
	}
}

/// [`serde`] helpers for [`BTreeMap<K, V>`].
///
/// [`BTreeMap<K, V>`]: std::collections::BTreeMap
pub mod btree_map {
	use std::collections::BTreeMap;

	use serde::{Deserialize, Deserializer};

	/// Deserializes a `BTreeMap<K, V>`, but treats empty arrays as `None`.
	pub fn deserialize_empty_as_none<'de, D, K, V>(
		deserializer: D,
	) -> Result<Option<BTreeMap<K, V>>, D::Error>
	where
		D: Deserializer<'de>,
		K: Deserialize<'de> + Ord,
		V: Deserialize<'de>,
	{
		let Some(map) = Option::<BTreeMap<K, V>>::deserialize(deserializer)? else {
			return Ok(None);
		};

		if map.is_empty() {
			return Ok(None);
		}

		Ok(Some(map))
	}
}

/// [`serde`] helpers for [`semver::Version`].
///
/// [`semver::Version`]: ::semver::Version
pub mod semver {
	use semver::Version;
	use serde::{de, Deserialize, Deserializer};

	/// Deserializes plugin version names submitted by GitHub actions.
	pub fn deserialize_plugin_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut version = <&'de str>::deserialize(deserializer)?;

		if let ("v", rest) = version.split_at(1) {
			version = rest;
		}

		version.parse::<Version>().map_err(de::Error::custom)
	}
}
