//! This module contains useful helper types for query parameters.

use derive_more::Display;
use serde::{Deserialize, Deserializer};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

/// An offset used for pagination.
#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// A limit on the amount of returned results from a request.
///
/// This will defaultu to `DEFAULT` (which is 100 by default), and max out at `MAX` (which is 1000
/// by default). These values can be overriden as necessary.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// A query parameter to decide a sorting order.
#[derive(Debug, Default, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortingOrder {
	/// Sort from lowest to highest.
	#[default]
	Ascending,

	/// Sort from highest to lowest.
	Descending,
}

impl SortingOrder {
	/// Returns a SQL keyword that can be used in an `ORDER BY` clause.
	pub const fn sql(&self) -> &'static str {
		match *self {
			SortingOrder::Ascending => " ASC ",
			SortingOrder::Descending => " DESC ",
		}
	}
}
