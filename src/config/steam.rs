use std::fmt::{self, Debug};
use std::path::PathBuf;

use crate::env::{self, Result};

/// Configuration for communicating with Steam.
pub struct Config {
	/// Steam [WebAPI] Key.
	///
	/// [WebAPI]: https://steamcommunity.com/dev
	pub api_key: String,

	/// Path to the DepotDownloader executable.
	pub workshop_downloader_path: Option<PathBuf>,

	/// Path to the directory where workshop files should be downloaded to.
	pub workshop_artifacts_path: Option<PathBuf>,
}

impl Config {
	pub fn new() -> Result<Self> {
		let api_key = env::get("STEAM_KEY")?;

		fn treat_empty_as_none(value: PathBuf) -> Option<PathBuf> {
			(value.as_os_str() != "").then_some(value)
		}

		let workshop_downloader_path = env::get("STEAM_WORKSHOP_DOWNLOADER_PATH")
			.ok()
			.and_then(treat_empty_as_none);

		let workshop_artifacts_path = env::get("STEAM_WORKSHOP_ARTIFACTS_PATH")
			.ok()
			.and_then(treat_empty_as_none);

		Ok(Self { api_key, workshop_downloader_path, workshop_artifacts_path })
	}
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut f = f.debug_struct("Config");

		f.field("api_key", &"*****");

		if let Some(path) = &self.workshop_downloader_path {
			f.field("workshop_downloader_path", &path.display());
		}

		if let Some(path) = &self.workshop_artifacts_path {
			f.field("workshop_artifacts_path", &path.display());
		}

		f.finish()
	}
}
