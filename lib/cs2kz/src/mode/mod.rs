//! CS2KZ officially supports two modes.
//!
//! While you can write your own modes as separate plugins, the API is only
//! concerned with the two official ones.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "utoipa")]
mod utoipa;

#[cfg(test)]
mod tests;

/// The two gamemodes officially supported by CS2KZ.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode
{
	/// The VNL gamemode.
	///
	/// As close to default CS2 movement as possible.
	Vanilla = 1,

	/// The CKZ gamemode.
	///
	/// A mix of KZTimer, SimpleKZ, and 1.6 KZ.
	Classic = 2,
}

impl Mode
{
	/// Checks whether `self` is [Vanilla].
	///
	/// [Vanilla]: Mode::Vanilla
	pub const fn is_vanilla(&self) -> bool
	{
		matches!(self, Self::Vanilla)
	}

	/// Checks whether `self` is [Classic].
	///
	/// [Classic]: Mode::Classic
	pub const fn is_classic(&self) -> bool
	{
		matches!(self, Self::Classic)
	}

	/// Returns a string representation of `self`.
	pub const fn as_str(&self) -> &'static str
	{
		match self {
			Self::Vanilla => "vanilla",
			Self::Classic => "classic",
		}
	}

	/// Returns a capitalized string representation of `self`.
	///
	/// This yields the same result as the [`fmt::Debug`] implementation, but is
	/// `const`.
	pub const fn as_str_capitalized(&self) -> &'static str
	{
		match self {
			Self::Vanilla => "Vanilla",
			Self::Classic => "Classic",
		}
	}

	/// Returns an abbreviated string representation of `self`.
	///
	/// This yields the same result as the [`fmt::Display`] implementation, but
	/// is `const`.
	pub const fn as_str_short(&self) -> &'static str
	{
		match self {
			Self::Vanilla => "VNL",
			Self::Classic => "CKZ",
		}
	}
}

impl fmt::Display for Mode
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.write_str(self.as_str_short())
	}
}

impl From<Mode> for u8
{
	fn from(value: Mode) -> Self
	{
		value as u8
	}
}

/// An error that can occur when converting from a `u8` to a [`Mode`].
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("`{0}` is not a known mode ID")]
pub struct UnknownModeID(pub u8);

impl TryFrom<u8> for Mode
{
	type Error = UnknownModeID;

	fn try_from(value: u8) -> Result<Self, Self::Error>
	{
		match value {
			1 => Ok(Self::Vanilla),
			2 => Ok(Self::Classic),
			n => Err(UnknownModeID(n)),
		}
	}
}

/// An error that can occur when parsing a string into a [`Mode`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error("`{0}` is not a known mode")]
pub struct UnknownMode(pub String);

impl FromStr for Mode
{
	type Err = UnknownMode;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = s.parse::<u8>() {
			return Self::try_from(int).map_err(|_| UnknownMode(s.to_owned()));
		}

		if s.eq_ignore_ascii_case("vnl") || s.eq_ignore_ascii_case("vanilla") {
			return Ok(Self::Vanilla);
		}

		if s.eq_ignore_ascii_case("ckz") || s.eq_ignore_ascii_case("classic") {
			return Ok(Self::Classic);
		}

		Err(UnknownMode(s.to_owned()))
	}
}
