use std::result::Result as StdResult;

use thiserror::Error as ThisError;
use tokio::io;

pub mod workshop;

pub mod player;
pub use player::Player;

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr while interacting with Steam.
#[derive(Debug, ThisError)]
pub enum Error {
	#[error("Error communicating with Steam: {0}")]
	Http(#[from] reqwest::Error),

	#[error("`{0}` is not a valid Workshop ID")]
	InvalidWorkshopID(u32),

	#[error("No steamcmd executable found.")]
	MissingSteamCMD,

	#[error("No workshop directory found.")]
	MissingWorkshopDirectory,

	#[error("Error executing SteamCMD{}", match .0 {
		None => String::new(),
		Some(err) => format!(" : {err}"),
	})]
	SteamCMD(Option<io::Error>),

	#[error("{0}")]
	IO(#[from] io::Error),
}
