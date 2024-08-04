//! A wrapper for validating and performing conversions on
//! [Valve's SteamID format][valve-docs].
//!
//! It does not currently support all the possible conversions, as it is mainly
//! focused on CS2.
//!
//! [valve-docs]: https://developer.valvesoftware.com/wiki/SteamID

use std::borrow::Borrow;
use std::num::{NonZero, ParseIntError};
use std::str::FromStr;
use std::{fmt, mem, ops};

use thiserror::Error;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "utoipa")]
mod utoipa;

#[cfg(test)]
mod tests;

/// This is a compile-time sanity check.
const _ASSERT_NPO: () = {
	assert!(
		mem::size_of::<SteamID>() == mem::size_of::<Option<SteamID>>(),
		"`SteamID` should be null-pointer optimized."
	);
};

/// The minimum value for a valid SteamID.
const MIN: u64 = 76561197960265729_u64;

/// The minimum value for a valid SteamID.
const MAX: u64 = 76561202255233023_u64;

/// Used for bit operations, see implementation below.
const MAGIC_OFFSET: u64 = MIN - 1;

/// A type for working with [Valve's SteamID format][valve-docs].
///
/// [valve-docs]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZero<u64>);

impl SteamID
{
	/// The minimum value for a valid [`SteamID`].
	pub const MIN: Self = unsafe { Self::new_unchecked(MIN) };

	/// The maximum value for a valid [`SteamID`].
	pub const MAX: Self = unsafe { Self::new_unchecked(MAX) };

	/// Creates a new [`SteamID`].
	///
	/// This function assumes a "SteamID64", which means a value within
	/// `SteamID::MIN..=SteamID::MAX`. If `value` is out of range, this function
	/// will return [`None`].
	pub const fn new(value: u64) -> Option<Self>
	{
		match value {
			MIN..=MAX => Some(unsafe { Self::new_unchecked(value) }),
			_ => None,
		}
	}

	/// Creates a new [`SteamID`] without checking that `value` is in-range.
	///
	/// # Safety
	///
	/// The caller must guarantee that `value` is within
	/// `SteamID::MIN..=SteamID::MAX`.
	pub const unsafe fn new_unchecked(value: u64) -> Self
	{
		debug_assert!(matches!(value, MIN..=MAX), "SteamID out of range");

		// SAFETY: The caller must guarantee that `value` is non-zero.
		Self(unsafe { NonZero::new_unchecked(value) })
	}

	/// Returns the `X` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn x(&self) -> u64
	{
		let x = self.0.get() >> 56;
		debug_assert!(matches!(x, 0 | 1), "SteamID X segment has an invalid value");
		x
	}

	/// Returns the `Y` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn y(&self) -> u64
	{
		let y = self.0.get() & 1;
		debug_assert!(matches!(y, 0 | 1), "SteamID Y segment has an invalid value");
		y
	}

	/// Returns the `Z` segment in `STEAM_X:Y:Z`.
	pub const fn z(&self) -> u64
	{
		(self.0.get() - MAGIC_OFFSET - self.y()) / 2
	}

	/// Returns the `SteamID` in its 64-bit representation.
	pub const fn as_u64(&self) -> u64
	{
		self.0.get()
	}

	/// Returns the `SteamID` in its 32-bit representation.
	pub const fn as_u32(&self) -> u32
	{
		let value = ((self.z() + self.y()) * 2) - self.y();

		debug_assert!(
			0 < value && value <= (u32::MAX as u64),
			"SteamID 32-bit representation has an invalid value"
		);

		value as u32
	}

	/// Returns the `SteamID` in its "Steam3ID" representation.
	pub fn as_id3(&self) -> String
	{
		format!("U:1:{}", self.as_u32())
	}

	/// Returns a `SteamID`, if the given `value` is in-range.
	pub const fn from_u32(value: u32) -> Option<Self>
	{
		Self::new((value as u64) + MAGIC_OFFSET)
	}

	/// Parses a [`SteamID`] in the standard format of `STEAM_X:Y:Z`.
	pub fn from_standard(value: impl AsRef<str>) -> Result<Self, ParseSteamIDError>
	{
		let mut value = value.as_ref();

		if value.starts_with("STEAM_") {
			value = value.trim_start_matches("STEAM_");
		} else {
			return Err(ParseSteamIDError::MissingPrefix);
		}

		let mut segments = value.split(':');

		match segments.next() {
			Some("0" | "1") => {}
			Some(_) => {
				return Err(ParseSteamIDError::InvalidX);
			}
			None => {
				return Err(ParseSteamIDError::MissingX);
			}
		}

		let y = segments
			.next()
			.ok_or(ParseSteamIDError::MissingY)?
			.parse::<u64>()
			.map_err(|err| ParseSteamIDError::InvalidY(Some(err)))?;

		if y > 1 {
			return Err(ParseSteamIDError::InvalidY(None));
		}

		let z = segments
			.next()
			.ok_or(ParseSteamIDError::MissingZ)?
			.parse::<u64>()
			.map_err(ParseSteamIDError::InvalidZ)?;

		if y == 0 && z == 0 {
			return Err(ParseSteamIDError::IsZero);
		}

		if (z + MAGIC_OFFSET) > MAX {
			return Err(ParseSteamIDError::OutOfRange);
		}

		Self::new(MAGIC_OFFSET | y | (z << 1)).ok_or(ParseSteamIDError::OutOfRange)
	}

	/// Parses a "Steam3ID" into a [`SteamID`].
	///
	/// The expected input format is `U:1:322356345`, optionally enclosed in
	/// `[]`.
	pub fn from_id3(value: impl AsRef<str>) -> Result<Self, ParseSteam3IDError>
	{
		let mut value = value.as_ref();

		match (value.starts_with('['), value.ends_with(']')) {
			(false, false) => {}
			(true, true) => {
				value = value.trim_start_matches('[').trim_end_matches(']');
			}
			(true, false) | (false, true) => {
				return Err(ParseSteam3IDError::InconsistentBrackets);
			}
		}

		let mut segments = value.split(':');

		let Some("U") = segments.next() else {
			return Err(ParseSteam3IDError::MissingAccountType);
		};

		let Some("1") = segments.next() else {
			return Err(ParseSteam3IDError::MissingOne);
		};

		let id = segments
			.next()
			.ok_or(ParseSteam3IDError::MissingID)?
			.parse::<u32>()
			.map_err(ParseSteam3IDError::InvalidID)?;

		Self::from_u32(id).ok_or(ParseSteam3IDError::OutOfRange)
	}
}

/// Potential errors that can occur when parsing a Steam3ID.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParseSteamIDError
{
	/// Every SteamID starts with `STEAM_`.
	#[error("missing `STEAM_` prefix")]
	MissingPrefix,

	/// The SteamID ended after the `STEAM_` prefix.
	#[error("missing `X` segment")]
	MissingX,

	/// The `X` segment was something other than 0 or 1.
	#[error("invalid `X` segment; expected 0 or 1")]
	InvalidX,

	/// The SteamID was missing the `Y` segment.
	#[error("missing `Y` segment")]
	MissingY,

	/// The `Y` segment was something other than 0 or 1.
	#[error("invalid `Y` segment; expected 0 or 1{}", match .0 {
		None => String::new(),
		Some(err) => format!(" ({err})"),
	})]
	InvalidY(Option<ParseIntError>),

	/// The SteamID was missing the `Z` segment.
	#[error("missing `Z` segment")]
	MissingZ,

	/// The `Z` segment was not a valid `u64`.
	#[error("invalid `Z` segment: {0}")]
	InvalidZ(ParseIntError),

	/// SteamIDs can't have a value of 0.
	#[error("is zero")]
	IsZero,

	/// SteamID value was otherwise out of bounds.
	#[error("64-bit SteamID out of range")]
	OutOfRange,
}

/// Potential errors that can occur when parsing a Steam3ID.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParseSteam3IDError
{
	/// Steam3IDs can optionally be enclosed by `[]`, e.g., `[U:1:322356345]`.
	///
	/// If such a string is passed, but it has exactly one of either the opening
	/// or closing bracket, that's a malformed ID.
	#[error("got one of, but not both, opening or closing bracket")]
	InconsistentBrackets,

	/// Steam3IDs contain an account type as their first segment.
	///
	/// For CS2 this is typically `U`.
	#[error("missing `U` segment specifying account type")]
	MissingAccountType,

	/// Steam3IDs _always_ have a `1` as their second segment.
	#[error("missing `1` segment")]
	MissingOne,

	/// Steam3IDs have the 32-bit ID representation as their third segment.
	#[error("missing ID segment")]
	MissingID,

	/// Parsing the ID segment failed, because it was not a valid `u32`.
	#[error("invalid 32-bit SteamID: {0}")]
	InvalidID(ParseIntError),

	/// Parsing the ID segment failed, because the value was out of range for a
	/// legal SteamID.
	#[error("32-bit SteamID out of range")]
	OutOfRange,
}

impl fmt::Display for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
	}
}

impl fmt::Binary for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl Borrow<u64> for SteamID
{
	fn borrow(&self) -> &u64
	{
		// SAFETY:
		//   1. `NonZero<T>` is marked `#[repr(transparent)]`, which means
		//      `NonZero<u64>` has the same layout as `u64`.
		//   2. `SteamID` is marked `#[repr(transparent)]`, which means it has the same
		//      layout as `NonZero<u64>`, and therefore as `u64`.
		//   3. We don't implement `BorrowMut<u64>` or otherwise expose a `&mut u64`,
		//      which would allow users to invalidate `NonZero<T>`'s invariants.
		unsafe { &*(&self.0 as *const NonZero<u64> as *const u64) }
	}
}

impl Borrow<NonZero<u64>> for SteamID
{
	fn borrow(&self) -> &NonZero<u64>
	{
		&self.0
	}
}

impl AsRef<u64> for SteamID
{
	fn as_ref(&self) -> &u64
	{
		self.borrow()
	}
}

impl AsRef<NonZero<u64>> for SteamID
{
	fn as_ref(&self) -> &NonZero<u64>
	{
		self.borrow()
	}
}

impl ops::Deref for SteamID
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		self.borrow()
	}
}

/// Helper macro for implementing repetetive traits.
macro_rules! impl_partial_ops {
	($t1:ty: [$($t2:ty),* $(,)?]) => {
		$(impl PartialEq<$t2> for $t1
		{
			fn eq(&self, other: &$t2) -> bool
			{
				<$t2 as PartialEq<$t2>>::eq(self.borrow(), other)
			}
		}

		impl PartialEq<$t1> for $t2
		{
			fn eq(&self, other: &$t1) -> bool
			{
				<$t2 as PartialEq<$t2>>::eq(self, other.borrow())
			}
		}

		impl PartialOrd<$t2> for $t1
		{
			fn partial_cmp(&self, other: &$t2) -> Option<::std::cmp::Ordering>
			{
				<$t2 as PartialOrd<$t2>>::partial_cmp(self.borrow(), other)
			}
		}

		impl PartialOrd<$t1> for $t2
		{
			fn partial_cmp(&self, other: &$t1) -> Option<::std::cmp::Ordering>
			{
				<$t2 as PartialOrd<$t2>>::partial_cmp(self, other.borrow())
			}
		})*
	};
}

impl_partial_ops!(SteamID: [u64, NonZero<u64>]);

impl From<SteamID> for u64
{
	fn from(value: SteamID) -> Self
	{
		*value
	}
}

impl From<SteamID> for NonZero<u64>
{
	fn from(value: SteamID) -> Self
	{
		value.0
	}
}

impl From<NonZero<u64>> for SteamID
{
	fn from(value: NonZero<u64>) -> Self
	{
		Self(value)
	}
}

/// An error that can occur when converting from a `u64` to a [`SteamID`].
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("`{0}` is out of range for a valid SteamID64")]
pub struct OutOfRangeSteamID64(pub u64);

impl TryFrom<u64> for SteamID
{
	type Error = OutOfRangeSteamID64;

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		Self::new(value).ok_or(OutOfRangeSteamID64(value))
	}
}

/// An error that can occur when converting from a `u32` to a [`SteamID`].
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("`{0}` is out of range for a valid SteamID32")]
pub struct OutOfRangeSteamID32(pub u32);

impl TryFrom<u32> for SteamID
{
	type Error = OutOfRangeSteamID32;

	fn try_from(value: u32) -> Result<Self, Self::Error>
	{
		Self::from_u32(value).ok_or(OutOfRangeSteamID32(value))
	}
}

impl TryFrom<NonZero<u32>> for SteamID
{
	type Error = OutOfRangeSteamID32;

	fn try_from(value: NonZero<u32>) -> Result<Self, Self::Error>
	{
		Self::try_from(value.get())
	}
}

/// Parsing a [`SteamID`] from a string failed.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum InvalidSteamID
{
	/// String could be parsed into an integer, but the integer was invalid.
	#[error(transparent)]
	InvalidU32(#[from] OutOfRangeSteamID32),

	/// String could be parsed into an integer, but the integer was invalid.
	#[error(transparent)]
	InvalidU64(#[from] OutOfRangeSteamID64),

	/// String could not be parsed as any known format.
	#[error("failed to parse SteamID; unrecognized format")]
	UnrecognizedSteamIDFormat,
}

impl FromStr for SteamID
{
	type Err = InvalidSteamID;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = s.parse::<u32>() {
			return Self::try_from(int).map_err(Into::into);
		}

		if let Ok(int) = s.parse::<u64>() {
			return Self::try_from(int).map_err(Into::into);
		}

		if let Ok(steam_id) = Self::from_standard(s) {
			return Ok(steam_id);
		}

		if let Ok(steam_id) = Self::from_id3(s) {
			return Ok(steam_id);
		}

		Err(InvalidSteamID::UnrecognizedSteamIDFormat)
	}
}
