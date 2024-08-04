//! This module contains extensions to [`std::time`].

use std::ops;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Extension trait for [`std::time::Duration`] which adds useful associated
/// constants.
#[sealed]
#[allow(dead_code)]
pub trait DurationExt
{
	/// One minute.
	const MINUTE: Duration = Duration::from_secs(60);

	/// One hour.
	const HOUR: Duration = Duration::from_secs(60 * 60);

	/// One day.
	const DAY: Duration = Duration::from_secs(60 * 60 * 24);

	/// One week.
	const WEEK: Duration = Duration::from_secs(60 * 60 * 24 * 7);

	/// One month (30 days).
	const MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30);

	/// One year (365 days).
	const YEAR: Duration = Duration::from_secs(60 * 60 * 24 * 365);
}

#[sealed]
impl DurationExt for Duration {}

/// A wrapper around [`std::time::Duration`] that ensures encoding/decoding
/// always happens in terms of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, utoipa::ToSchema)]
#[schema(value_type = f64)]
pub struct Seconds(pub Duration);

impl ops::Deref for Seconds
{
	type Target = Duration;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl ops::DerefMut for Seconds
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		&mut self.0
	}
}

impl From<Duration> for Seconds
{
	fn from(value: Duration) -> Self
	{
		Self(value)
	}
}

impl From<Seconds> for Duration
{
	fn from(value: Seconds) -> Self
	{
		value.0
	}
}

impl Serialize for Seconds
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_secs_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer)
			.map(Duration::from_secs_f64)
			.map(Self)
	}
}

crate::macros::sqlx_scalar_forward!(Seconds as f64 => {
	encode: |self| { self.0.as_secs_f64() },
	decode: |secs| { Self(Duration::from_secs_f64(secs)) },
});
