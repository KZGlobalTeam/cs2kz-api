//! This module contains useful helper types for query parameters.

use serde::{Deserialize, Deserializer};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

/// An offset used for pagination.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(pub i64);

impl<'de> Deserialize<'de> for Offset {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<i64>::deserialize(deserializer)
			.map(Option::unwrap_or_default)
			.map(Self)
	}
}

impl<'s> ToSchema<'s> for Offset {
	fn schema() -> (&'s str, RefOr<Schema>) {
		(
			"Offset",
			Schema::Object(
				ObjectBuilder::new()
					.description(Some("used for pagination"))
					.schema_type(SchemaType::Number)
					.minimum(Some(i64::MIN as f64))
					.maximum(Some(i64::MAX as f64))
					.default(Some(0.into()))
					.build(),
			)
			.into(),
		)
	}
}

/// A limit on the amount of returned results from a request.
///
/// This will defaultu to `DEFAULT` (which is 100 by default), and max out at `MAX` (which is 1000
/// by default). These values can be overriden as necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Limit<const MAX: u64 = 1000, const DEFAULT: u64 = 100>(pub u64);

impl<const MAX: u64, const DEFAULT: u64> Default for Limit<MAX, DEFAULT> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl From<Limit> for usize {
	#[allow(clippy::cast_possible_truncation)]
	fn from(value: Limit) -> Self {
		value.0 as usize
	}
}

impl<'de, const MAX: u64, const DEFAULT: u64> Deserialize<'de> for Limit<MAX, DEFAULT> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error;

		match Option::deserialize(deserializer).map(|value| value.unwrap_or(DEFAULT))? {
			value if value <= MAX => Ok(Self(value)),
			value => Err(Error::custom(format_args!(
				"invalid limit `{value}`; cannot exceed `{MAX}`"
			))),
		}
	}
}

impl<'s, const MAX: u64, const DEFAULT: u64> ToSchema<'s> for Limit<MAX, DEFAULT> {
	fn schema() -> (&'s str, RefOr<Schema>) {
		(
			"Limit",
			Schema::Object(
				ObjectBuilder::new()
					.description(Some("limits the amount of returned values"))
					.schema_type(SchemaType::Number)
					.minimum(Some(0 as f64))
					.build(),
			)
			.into(),
		)
	}
}
