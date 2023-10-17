// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

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
}
