//! CS2KZ officially supports a set of gameplay styles that can be combined in
//! addition to a [mode].
//!
//! This module contains bitflags for these styles.
//!
//! [mode]: crate::mode

use std::str::FromStr;
use std::{fmt, ops};

use thiserror::Error;

mod iter;

#[doc(inline)]
pub use iter::Iter;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "utoipa")]
mod utoipa;

/// All official gameplay styles included in the CS2KZ plugin.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Styles(u32);

impl Styles
{
	/// No styles.
	pub const NONE: Self = Self(0);

	/// The "ABH" style.
	pub const AUTO_BHOP: Self = Self(1 << 0);

	/// All styles.
	pub const ALL: Self = Self(1 << 0);

	/// Create new bitflags from a raw integer value.
	///
	/// # Panics
	///
	/// This function will panic if `value` contains any unknown bits.
	pub const fn new(value: u32) -> Self
	{
		assert!(value & Self::ALL.0 == value, "invalid style bits");
		Self(value)
	}

	/// Create new bitflags from a raw integer value.
	pub const fn new_checked(value: u32) -> Option<Self>
	{
		if value & Self::ALL.0 == value {
			Some(Self(value))
		} else {
			None
		}
	}

	/// Returns the underlying integer value.
	pub const fn bits(self) -> u32
	{
		self.0
	}

	/// If `self` currently has 1 bit set, this function will return the name
	/// of that bit.
	pub const fn name(self) -> Option<&'static str>
	{
		match self {
			Self::AUTO_BHOP => Some("auto_bhop"),
			_ => None,
		}
	}

	/// Checks if `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool
	{
		(self.0 & other.0) == other.0
	}

	/// Creates an iterator over the style bits.
	pub const fn iter_bits(self) -> Iter<u32>
	{
		Iter::new(self).bits()
	}

	/// Creates an iterator over the style names.
	pub const fn iter_names(self) -> Iter<str>
	{
		Iter::new(self).names()
	}
}

impl fmt::Display for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_list().entries(self.iter_names()).finish()
	}
}

impl fmt::Binary for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl ops::Deref for Styles
{
	type Target = u32;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl ops::BitOr for Styles
{
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 | rhs.0)
	}
}

impl ops::BitOrAssign for Styles
{
	fn bitor_assign(&mut self, rhs: Self)
	{
		*self = *self | rhs;
	}
}

impl ops::BitAnd for Styles
{
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 & rhs.0)
	}
}

impl ops::BitAndAssign for Styles
{
	fn bitand_assign(&mut self, rhs: Self)
	{
		*self = *self & rhs;
	}
}

impl ops::BitXor for Styles
{
	type Output = Self;

	fn bitxor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 ^ rhs.0)
	}
}

impl ops::BitXorAssign for Styles
{
	fn bitxor_assign(&mut self, rhs: Self)
	{
		*self = *self ^ rhs;
	}
}

/// An error that can occur when parsing a string into [`Styles`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error("unknown style `{0}`")]
pub struct UnknownStyle(pub String);

impl FromStr for Styles
{
	type Err = UnknownStyle;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"auto_bhop" => Ok(Self::AUTO_BHOP),
			unknown => Err(UnknownStyle(unknown.to_owned())),
		}
	}
}
