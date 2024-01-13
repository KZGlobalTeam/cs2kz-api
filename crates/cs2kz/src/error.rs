use std::result::Result as StdResult;

use thiserror::Error;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
	#[error("`{value}` is out of bounds for a valid SteamID.")]
	OutOfBoundsSteamID { value: u64 },

	#[error("`{value}` was assumed to be a Steam3ID, but was invalid: {reason}")]
	InvalidSteam3ID { value: String, reason: &'static str },

	#[error("`{value}` was assumed to be a SteamID, but was invalid: {reason}")]
	InvalidSteamID { value: String, reason: &'static str },

	#[error("`{value}` is not a valid Mode ID.")]
	InvalidModeID { value: u8 },

	#[error("`{value}` is not a valid Mode.")]
	InvalidMode { value: String },

	#[error("`{value}` is not a valid Style ID.")]
	InvalidStyleID { value: u8 },

	#[error("`{value}` is not a valid Style.")]
	InvalidStyle { value: String },

	#[error("`{value}` is not a valid Jumpstat ID.")]
	InvalidJumpstatID { value: u8 },

	#[error("`{value}` is not a valid Jumpstat.")]
	InvalidJumpstat { value: String },

	#[error("`{value}` is not a valid Tier.")]
	InvalidTier { value: String },
}
