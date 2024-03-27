//! Configuration that is loaded at startup and then used throughout the application.

use std::fmt::{self, Debug};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use serde::Deserialize;
use url::Url;

/// This struct is initialized once when the API starts up, and its values are read from the
/// environment.
#[derive(Clone, Deserialize)]
pub struct Config {
	/// The IP address the HTTP server should listen on.
	#[serde(rename = "api_ip")]
	pub ip_addr: IpAddr,

	/// The port the HTTP server should listen on.
	#[serde(rename = "api_port")]
	pub port: u16,

	/// URL of the database the API should connect to.
	pub database_url: Url,

	/// URL for connecting to MySql as the root user.
	///
	/// This is necessary in integration tests to create a separate database for
	/// each test.
	#[cfg(test)]
	pub database_admin_url: Url,

	/// JWT secret for encoding / decoding tokens.
	pub jwt_secret: String,

	/// The API's public URL.
	pub public_url: Url,

	/// Domain to use for authentication cookies.
	pub domain: String,

	/// Steam WebAPI Key.
	pub steam_api_key: String,

	/// Directory for storing Steam Workshop artifacts.
	pub workshop_artifacts_path: Option<PathBuf>,

	/// Path to `DepotDownloader` executable.
	pub depot_downloader_path: Option<PathBuf>,
}

impl Config {
	/// Parses a [`Config`] instance from the environment.
	pub fn new() -> envy::Result<Self> {
		envy::from_env()
	}

	/// Returns a full [`SocketAddr`] for where the HTTP server should listen on.
	pub fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(self.ip_addr, self.port)
	}
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut f = f.debug_struct("Config");

		f.field("address", &self.socket_addr())
			.field("database_url", &"*****")
			.field("jwt_secret", &"*****")
			.field("public_url", &self.public_url.as_str())
			.field("steam_api_key", &"*****");

		if let Some(workshop_artifacts_path) = &self.workshop_artifacts_path {
			f.field("workshop_artifacts_path", &workshop_artifacts_path.display());
		}

		if let Some(depot_downloader_path) = &self.depot_downloader_path {
			f.field("depot_downloader_path", &depot_downloader_path.display());
		}

		f.finish()
	}
}
