//! This module contains extensions for [`serde`].

use serde::{Deserialize, Deserializer};

use crate::util::IsEmpty;

/// Deserializes a collection and makes sure it isn't empty.
pub fn deserialize_non_empty<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
	T: IsEmpty + Deserialize<'de>,
	D: Deserializer<'de>,
{
	T::deserialize(deserializer).and_then(|v| match v.is_empty() {
		false => Ok(v),
		true => Err(serde::de::Error::invalid_length(0, &"1 or more")),
	})
}

/// Deserializes an `Option<T>` but treats `Some(<empty>)` as `None`.
pub fn deserialize_empty_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
	T: IsEmpty,
	Option<T>: Deserialize<'de>,
	D: Deserializer<'de>,
{
	Option::<T>::deserialize(deserializer).map(|opt| opt.filter(|v| !v.is_empty()))
}

#[cfg(test)]
mod tests
{
	use serde_json::json;

	#[test]
	fn deserialize_non_empty()
	{
		let empty = json!([]);
		let result = super::deserialize_non_empty::<Vec<i32>, _>(empty);

		assert!(result.is_err());
	}

	#[test]
	fn deserialize_empty_as_none() -> color_eyre::Result<()>
	{
		let empty = json!(null);
		let result = super::deserialize_empty_as_none::<String, _>(empty)?;

		assert!(result.is_none());

		let empty = json!("");
		let result = super::deserialize_empty_as_none::<String, _>(empty)?;

		assert!(result.is_none());

		let empty = json!("foo");
		let result = super::deserialize_empty_as_none::<String, _>(empty)?;

		assert_eq!(result.as_deref(), Some("foo"));

		Ok(())
	}
}
