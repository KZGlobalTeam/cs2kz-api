use std::num::{NonZeroU32, NonZeroU64};

/// A [SteamID].
///
/// This acts as a transparent wrapper over a regular [`u32`] with convenience methods and trait
/// implementations when working with [Steam] accounts.
///
/// [Steam]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZeroU32);

impl SteamID {
	/// The minimum value for any valid [`SteamID`].
	// SAFETY: this is constructed at compile-time and non-zero
	pub const MIN: Self = Self(unsafe { NonZeroU32::new_unchecked(1) });

	/// The maximum value for any valid [`SteamID`].
	// SAFETY: this is constructed at compile-time and non-zero
	pub const MAX: Self = Self(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

	/// Used for converting between 32 and 64 bit.
	const MAGIC_OFFSET: u64 = 76561197960265728_u64;

	/// The 32-bit representation of this [SteamID].
	#[inline]
	pub const fn as_u32(&self) -> NonZeroU32 {
		self.0
	}

	/// The 64-bit representation of this [SteamID].
	#[inline]
	pub const fn as_u64(&self) -> NonZeroU64 {
		// SAFETY: `Self::MAGIC_OFFSET` is non-zero, therefore the entire expression has to
		// be non-zero
		unsafe { NonZeroU64::new_unchecked(self.0.get() as u64 + Self::MAGIC_OFFSET) }
	}
}
