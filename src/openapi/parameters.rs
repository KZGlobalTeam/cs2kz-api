//! Generic query parameter types.

use derive_more::{Deref, Display};
use serde::{Deserialize, Deserializer};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

/// An "offset" query parameter used for pagination.
#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref)]
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
					.minimum(Some(f64::MIN))
					.maximum(Some(f64::MAX))
					.default(Some(0.into()))
					.build(),
			)
			.into(),
		)
	}
}

/// An "limit" query parameter used for pagination.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref)]
pub struct Limit<const MAX: u64 = 1000, const DEFAULT: u64 = 100>(pub u64);

impl<const MAX: u64, const DEFAULT: u64> Default for Limit<MAX, DEFAULT> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl From<Limit> for usize {
	#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
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
					.minimum(Some(0.0))
					.build(),
			)
			.into(),
		)
	}
}

/// An "sorting order" query parameter used for controlling the order of returned results.
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortingOrder {
	/// Sort results in ascending order (default).
	#[default]
	Ascending,

	/// Sort results in descending order.
	Descending,
}

impl SortingOrder {
	/// Generates the appropriate SQL keyword for an `ORDER BY` query.
	pub const fn sql(&self) -> &'static str {
		match self {
			SortingOrder::Ascending => " ASC ",
			SortingOrder::Descending => " DESC ",
		}
	}
}
