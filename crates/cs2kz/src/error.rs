//! Errors produced by this crate.
//!
//! Any error types used in the public API of this crate are defined in this module.

use std::result::Result as StdResult;

use thiserror::Error;

/// A [`Result`] with this crate's [`Error`] type.
///
/// [`Result`]: std::result::Result
/// [`Error`]: enum@Error
pub type Result<T> = StdResult<T, Error>;

/// The error type returned by any fallible function in this crate.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
	/// Parsing a [`SteamID`] failed.
	///
	/// [`SteamID`]: crate::SteamID
	#[error("invalid SteamID: {reason}")]
	InvalidSteamID {
		/// Why the parsing failed.
		reason: &'static str,
	},

	/// Parsing a [`Mode`] failed.
	///
	/// [`Mode`]: crate::Mode
	#[error("invalid mode")]
	InvalidMode,

	/// Parsing a [`Style`] failed.
	///
	/// [`Style`]: crate::Style
	#[error("invalid style")]
	InvalidStyle,

	/// Parsing a [`Tier`] failed.
	///
	/// [`Tier`]: crate::Tier
	#[error("invalid tier")]
	InvalidTier,

	/// Parsing a [`JumpType`] failed.
	///
	/// [`JumpType`]: crate::JumpType
	#[error("invalid jump type")]
	InvalidJumpType,

	/// Parsing a [`GlobalStatus`] failed.
	///
	/// [`GlobalStatus`]: crate::GlobalStatus
	#[error("invalid global status")]
	InvalidGlobalStatus,

	/// Parsing a [`RankedStatus`] failed.
	///
	/// [`RankedStatus`]: crate::RankedStatus
	#[error("invalid ranked status")]
	InvalidRankedStatus,
}

#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{Serialize, Serializer};

	use super::Error;

	impl Serialize for Error {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.to_string().serialize(serializer)
		}
	}
}
