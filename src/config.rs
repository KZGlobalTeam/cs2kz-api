//! Runtime configuration for the API.
//!
//! When starting up, the API will read configuration values from the environment to construct a
//! [`Config`]. It will then be part of the global application state.

use std::env;
use std::error::Error as StdError;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
use derive_more::Debug;
use url::Url;

/// The API's runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
	/// The address the API server will be listening on.
	#[debug("{addr}")]
	pub addr: SocketAddr,

	/// The database URL the API will connect to.
	#[debug("{}", database_url.as_str())]
	pub database_url: Url,

	/// The public URL of the API.
	#[debug("{}", public_url.as_str())]
	pub public_url: Url,

	/// The `Domain` field on cookies set by the API.
	#[debug("{cookie_domain}")]
	pub cookie_domain: String,

	/// Steam Web API key.
	///
	/// Get yours here: <https://steamcommunity.com/dev/apikey>
	#[debug("*****")]
	pub steam_api_key: String,

	/// Path to a directory where downloaded Workshop assets should be stored.
	#[cfg(not(feature = "production"))]
	pub workshop_artifacts_path: Option<PathBuf>,

	/// Path to a directory where downloaded Workshop assets should be stored.
	#[cfg(feature = "production")]
	pub workshop_artifacts_path: PathBuf,

	/// Path to the [DepotDownloader] executable.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(not(feature = "production"))]
	pub depot_downloader_path: Option<PathBuf>,

	/// Path to the [DepotDownloader] executable.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(feature = "production")]
	pub depot_downloader_path: PathBuf,

	/// JWT secret for encoding/decoding tokens.
	#[debug("*****")]
	pub jwt_secret: String,
}

impl Config {
	/// Creates a new [`Config`] by reading environment variables.
	pub fn new() -> anyhow::Result<Self> {
		tracing::debug!("loading configuration from environment");

		let ip_addr = parse_from_env("KZ_API_IP")?;
		let port = parse_from_env("KZ_API_PORT")?;
		let addr = SocketAddr::new(ip_addr, port);
		let database_url = parse_from_env("DATABASE_URL")?;
		let public_url = parse_from_env("KZ_API_PUBLIC_URL")?;
		let cookie_domain = parse_from_env("KZ_API_COOKIE_DOMAIN")?;
		let steam_api_key = parse_from_env("STEAM_WEB_API_KEY")?;

		#[cfg(not(feature = "production"))]
		let workshop_artifacts_path = parse_from_env_opt("KZ_API_WORKSHOP_PATH")?;

		#[cfg(feature = "production")]
		let workshop_artifacts_path = parse_from_env("KZ_API_WORKSHOP_PATH")?;

		#[cfg(not(feature = "production"))]
		let depot_downloader_path = parse_from_env_opt("DEPOT_DOWNLOADER_PATH")?;

		#[cfg(feature = "production")]
		let depot_downloader_path = parse_from_env("DEPOT_DOWNLOADER_PATH")?;

		let jwt_secret = parse_from_env("KZ_API_JWT_SECRET")?;

		Ok(Self {
			addr,
			database_url,
			public_url,
			cookie_domain,
			steam_api_key,
			workshop_artifacts_path,
			depot_downloader_path,
			jwt_secret,
		})
	}
}

/// Parses a value from the environment.
fn parse_from_env<T>(var: &str) -> anyhow::Result<T>
where
	T: FromStr,
	T::Err: StdError + Send + Sync + 'static,
{
	let value = env::var(var).with_context(|| format!("missing `{var}` environment variable"))?;

	if value.is_empty() {
		anyhow::bail!("`{var}` cannot be empty");
	}

	<T as FromStr>::from_str(&value).with_context(|| format!("failed to parse `{var}`"))
}

/// Parses a value from the environment.
///
/// Returns `Ok(None)` if the value does not exist, and `Err` if the value does exist, and parsing
/// it failed.
#[cfg(not(feature = "production"))]
fn parse_from_env_opt<T>(var: &str) -> anyhow::Result<Option<T>>
where
	T: FromStr,
	T::Err: StdError + Send + Sync + 'static,
{
	let Some(value) = env::var(var).ok() else {
		return Ok(None);
	};

	if value.is_empty() {
		return Ok(None);
	}

	<T as FromStr>::from_str(&value)
		.map(Some)
		.with_context(|| format!("failed to parse `{var}`"))
}
