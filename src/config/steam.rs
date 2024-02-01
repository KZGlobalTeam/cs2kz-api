use std::path::PathBuf;

use smart_debug::SmartDebug;

use super::{get_env_var, Result};

/// Configuration for communicating with Steam.
#[derive(SmartDebug)]
pub struct Config {
	/// Steam [WebAPI] Key.
	///
	/// [WebAPI]: https://steamcommunity.com/dev
	#[debug("…")]
	pub api_key: String,

	/// Path to the DepotDownloader executable.
	pub workshop_downloader_path: Option<PathBuf>,

	/// Path to the directory where workshop files should be downloaded to.
	pub steam_workshop_path: Option<PathBuf>,
}

impl Config {
	pub fn new() -> Result<Self> {
		let api_key = get_env_var("KZ_API_STEAM_API_KEY")?;

		let workshop_downloader_path = get_env_var("KZ_API_STEAM_WORKSHOP_DOWNLOADER_PATH")
			.ok()
			.and_then(treat_empty_as_none);

		let steam_workshop_path = get_env_var("KZ_API_STEAM_WORKSHOP_PATH")
			.ok()
			.and_then(treat_empty_as_none);

		Ok(Self { api_key, workshop_downloader_path, steam_workshop_path })
	}
}

fn treat_empty_as_none(value: PathBuf) -> Option<PathBuf> {
	(value.as_os_str() != "").then_some(value)
}
