use {std::result::Result as StdResult, thiserror::Error as ThisError};

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, ThisError)]
pub enum Error {
	/// An invalid input was passed to a [`SteamID`](crate::SteamID) constructor.
	#[error("`{}` is not a valid SteamID.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidSteamID { input: String, reason: Option<String> },

	#[error("`{}` is not a valid Mode.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidMode { input: String, reason: Option<String> },

	#[error("`{}` is not a valid Style.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidStyle { input: String, reason: Option<String> },

	#[error("`{}` is not a valid Jumpstat.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidJumpstat { input: String, reason: Option<String> },

	#[error("`{}` is not a valid Tier.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidTier { input: String, reason: Option<String> },

	#[error("`{}` is not a valid Runtype.{}", input, match reason {
		None => String::new(),
		Some(reason) => format!(" ({reason})"),
	})]
	InvalidRuntype { input: String, reason: Option<String> },
}
