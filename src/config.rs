//! Configuration that is loaded at startup and then used throughout the application.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use derive_more::Debug;
use serde::Deserialize;
use url::Url;

/// This struct is initialized once when the API starts up, and its values are read from the
/// environment.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	/// The IP address the HTTP server should listen on.
	#[serde(rename = "api_ip")]
	#[debug("{}", self.socket_addr())]
	pub ip_addr: IpAddr,

	/// The port the HTTP server should listen on.
	#[serde(rename = "api_port")]
	#[debug(skip)]
	pub port: u16,

	/// URL of the database the API should connect to.
	#[debug("*****")]
	pub database_url: Url,

	/// URL for connecting to MySql as the root user.
	///
	/// This is necessary in integration tests to create a separate database for
	/// each test.
	#[cfg(test)]
	#[debug("*****")]
	pub database_admin_url: Url,

	/// JWT secret for encoding / decoding tokens.
	#[debug("*****")]
	pub jwt_secret: String,

	/// The API's public URL.
	#[debug("{}", public_url.as_str())]
	pub public_url: Url,

	/// Domain to use for authentication cookies.
	#[debug("{domain}")]
	pub domain: String,

	/// Steam WebAPI Key.
	#[debug("*****")]
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
	pub const fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(self.ip_addr, self.port)
	}
}
