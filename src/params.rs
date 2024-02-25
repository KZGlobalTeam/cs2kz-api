use serde::{Deserialize, Deserializer};
use utoipa::ToSchema;

/// Utility type for extracting a "limit" query parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type, ToSchema)]
#[sqlx(transparent)]
pub struct Limit<const DEFAULT: u64 = 100, const MAX: u64 = 1000>(pub u64);

impl<const DEFAULT: u64, const MAX: u64> Default for Limit<DEFAULT, MAX> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl From<Limit> for usize {
	// this will never truncate on a 64-bit platform
	#[allow(clippy::cast_possible_truncation)]
	fn from(value: Limit) -> Self {
		value.0 as _
	}
}

impl<'de, const DEFAULT: u64, const MAX: u64> Deserialize<'de> for Limit<DEFAULT, MAX> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::{Error as E, Unexpected as U};

		match Option::<u64>::deserialize(deserializer)? {
			None => Ok(Self(DEFAULT)),
			Some(limit) if limit <= MAX => Ok(Self(limit)),
			Some(too_high) => Err(E::invalid_value(U::Unsigned(too_high), &"smaller value")),
		}
	}
}

/// Utility type for extracting an "offset" query parameter.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(default, transparent)]
#[sqlx(transparent)]
pub struct Offset(pub i64);
