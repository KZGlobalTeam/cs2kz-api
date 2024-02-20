use serde::{Deserialize, Deserializer};

pub fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	Option::<String>::deserialize(deserializer).map(|value| match value.as_deref() {
		None | Some("") => None,
		Some(_) => value,
	})
}

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
