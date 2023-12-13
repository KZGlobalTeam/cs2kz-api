//! This module holds utility functions for [`serde`].
//!
//! These are mainly alternative ways of (de)serializing various types.

#![allow(missing_docs)]

pub mod duration_as_secs {
	use std::time::Duration;

	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
		duration.as_secs().serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
		<u64 as Deserialize<'de>>::deserialize(deserializer).map(Duration::from_secs)
	}
}
