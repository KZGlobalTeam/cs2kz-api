use serde::{Deserialize, Deserializer};

pub mod duration {
	pub mod as_secs {
		use std::time::Duration;

		use serde::{Deserialize, Deserializer, Serialize, Serializer};

		pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			duration.as_secs().serialize(serializer)
		}

		pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
		where
			D: Deserializer<'de>,
		{
			u64::deserialize(deserializer).map(Duration::from_secs)
		}
	}
}

/// Deserializes an `Option<String>` such that an empty string is treated as `None`.
pub fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	Option::<String>::deserialize(deserializer).map(|value| match value.as_deref() {
		None | Some("") => None,
		Some(_) => value,
	})
}

/// Deserializes a `Vec<T>` but rejects if there are 0 elements.
pub fn deserialize_non_empty_vec<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
	T: Deserialize<'de>,
	D: Deserializer<'de>,
{
	use serde::de::Error as E;

	let vec = Vec::<T>::deserialize(deserializer)?;

	if vec.is_empty() {
		return Err(E::invalid_length(0, &"non-zero"));
	}

	Ok(vec)
}
