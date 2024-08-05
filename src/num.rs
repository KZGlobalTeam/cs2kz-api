//! This module contains extensions for [`std::num`].

use std::{cmp, ops};

use serde::{Deserialize, Deserializer, Serialize};

/// A u64 with custom default & max value.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ClampedU64<const DEFAULT: u64 = 0, const MAX: u64 = { u64::MAX }>(u64);

impl<const DEFAULT: u64, const MAX: u64> ClampedU64<DEFAULT, MAX>
{
	/// Create a new [`ClampedU64`].
	///
	/// This will truncate `value` to `MAX` if necessary.
	pub const fn new(value: u64) -> Self
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Self(if value > MAX { MAX } else { value })
	}
}

impl<const DEFAULT: u64, const MAX: u64> Default for ClampedU64<DEFAULT, MAX>
{
	fn default() -> Self
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Self(cmp::min(DEFAULT, MAX))
	}
}

impl<const DEFAULT: u64, const MAX: u64> From<u64> for ClampedU64<DEFAULT, MAX>
{
	fn from(value: u64) -> Self
	{
		Self::new(value)
	}
}

impl<const DEFAULT: u64, const MAX: u64> From<ClampedU64<DEFAULT, MAX>> for u64
{
	fn from(ClampedU64(value): ClampedU64<DEFAULT, MAX>) -> Self
	{
		value
	}
}

impl<const DEFAULT: u64, const MAX: u64> ops::Deref for ClampedU64<DEFAULT, MAX>
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl<'de, const DEFAULT: u64, const MAX: u64> Deserialize<'de> for ClampedU64<DEFAULT, MAX>
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Ok(Option::<u64>::deserialize(deserializer)?
			.map(Self::new)
			.unwrap_or_default())
	}
}

#[cfg(test)]
mod tests
{
	use super::*;

	#[test]
	fn default()
	{
		assert_eq!(<ClampedU64>::default().0, 0);
		assert_eq!(ClampedU64::<1>::default().0, 1);
	}

	#[test]
	fn new()
	{
		assert_eq!(ClampedU64::<0, 10>::new(10).0, 10);
		assert_eq!(ClampedU64::<0, 10>::new(11).0, 10);
	}
}
