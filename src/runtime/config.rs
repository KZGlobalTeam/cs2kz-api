//! This module contains the [`Config`] struct - a set of configuration options
//! that will be read from the environment on startup.
//!
//! See the `.env.example` file in the root of the repository for all the
//! relevant variables and example values.

use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fmt};

use thiserror::Error;
use url::Url;

/// The API's runtime configuration.
#[derive(Clone, clap::Parser)]
pub struct Config
{
	/// The public URL.
	///
	/// This might be sent to other APIs or be used for building URLs.
	#[arg(long, env = "KZ_API_PUBLIC_URL")]
	pub public_url: Url,

	/// Database connection URL.
	#[arg(long, env)]
	pub database_url: Url,

	/// Value to use for the `Domain` field on HTTP cookies.
	#[arg(long, env = "KZ_API_COOKIE_DOMAIN")]
	pub cookie_domain: String,

	/// Steam Web API key.
	#[arg(long, env = "STEAM_WEB_API_KEY")]
	pub steam_api_key: String,

	/// Base64 secret for encoding/decoding JWTs.
	#[arg(long, env = "KZ_API_JWT_SECRET")]
	pub jwt_secret: String,

	/// Path to a directory that can be used for storing Steam workshop assets.
	#[cfg(feature = "production")]
	#[arg(long, env = "KZ_API_WORKSHOP_PATH")]
	pub workshop_artifacts_path: PathBuf,

	/// Path to a directory that can be used for storing Steam workshop assets.
	#[cfg(not(feature = "production"))]
	#[arg(long, env = "KZ_API_WORKSHOP_PATH")]
	pub workshop_artifacts_path: Option<PathBuf>,

	/// Path to a [DepotDownloader] executable.
	///
	/// This can be used to download things from the Steam workshop.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(feature = "production")]
	#[arg(long, env)]
	pub depot_downloader_path: PathBuf,

	/// Path to a [DepotDownloader] executable.
	///
	/// This can be used to download things from the Steam workshop.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(not(feature = "production"))]
	#[arg(long, env)]
	pub depot_downloader_path: Option<PathBuf>,
}

/// Error that can occur while initializing the API's [`Config`].
#[derive(Debug, Error)]
pub enum InitializeConfigError
{
	/// A required environment variable was not found or invalid
	/// UTF-8.
	#[error("failed to read environment variable `{var}`: {source}")]
	Env
	{
		/// The environment variable we tried to read.
		var: &'static str,

		/// The original error we got from [`std::env::var()`] when we tried to
		/// read a value.
		source: env::VarError,
	},

	/// A required configuration option was empty.
	#[error("`{var}` cannot be empty")]
	EmptyValue
	{
		/// The environment variable we read.
		var: &'static str,
	},

	/// A required configuration option could not be parsed into the required
	/// type.
	#[error("failed to parse configuration value `{var}`: {source}")]
	Parse
	{
		/// The environment variable containing the value.
		var: &'static str,

		/// The parsing error.
		source: Box<dyn std::error::Error + Send + Sync + 'static>,
	},
}

impl Config
{
	/// Initializes a [`Config`] by reading and parsing environment variables.
	#[tracing::instrument(err(Debug))]
	pub fn new() -> Result<Self, InitializeConfigError>
	{
		let public_url = parse_from_env::<Url>("KZ_API_PUBLIC_URL")?;
		let database_url = parse_from_env::<Url>("DATABASE_URL")?;
		let cookie_domain = parse_from_env::<String>("KZ_API_COOKIE_DOMAIN")?;
		let steam_api_key = parse_from_env::<String>("STEAM_WEB_API_KEY")?;
		let jwt_secret = parse_from_env::<String>("KZ_API_JWT_SECRET")?;

		#[cfg(feature = "production")]
		let workshop_artifacts_path = parse_from_env::<PathBuf>("KZ_API_WORKSHOP_PATH")?;

		#[cfg(not(feature = "production"))]
		let workshop_artifacts_path = parse_from_env_opt::<PathBuf>("KZ_API_WORKSHOP_PATH")?;

		#[cfg(feature = "production")]
		let depot_downloader_path = parse_from_env::<PathBuf>("DEPOT_DOWNLOADER_PATH")?;

		#[cfg(not(feature = "production"))]
		let depot_downloader_path = parse_from_env_opt::<PathBuf>("DEPOT_DOWNLOADER_PATH")?;

		Ok(Self {
			public_url,
			database_url,
			cookie_domain,
			steam_api_key,
			jwt_secret,
			workshop_artifacts_path,
			depot_downloader_path,
		})
	}
}

impl fmt::Debug for Config
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Config")
			.field("public_url", &format_args!("{:?}", self.public_url.as_str()))
			.field("database_url", &format_args!("{:?}", self.database_url.as_str()))
			.field("cookie_domain", &self.cookie_domain)
			.field("steam_api_key", &"*****")
			.field("jwt_secret", &"*****")
			.field("workshop_artifacts_path", &self.workshop_artifacts_path)
			.field("depot_downloader_path", &self.depot_downloader_path)
			.finish_non_exhaustive()
	}
}

/// Reads and parses an environment variable.
fn parse_from_env<T>(var: &'static str) -> Result<T, InitializeConfigError>
where
	T: FromStr<Err: std::error::Error + Send + Sync + 'static>,
{
	let value = env::var(var).map_err(|source| InitializeConfigError::Env { var, source })?;

	if value.is_empty() {
		return Err(InitializeConfigError::EmptyValue { var });
	}

	value
		.parse::<T>()
		.map_err(|error| InitializeConfigError::Parse { var, source: Box::new(error) })
}

/// Reads and parses an environment variable.
///
/// Returns [`None`] if a variable does not exist or is empty.
#[cfg(not(feature = "production"))]
fn parse_from_env_opt<T>(var: &'static str) -> Result<Option<T>, InitializeConfigError>
where
	T: FromStr<Err: std::error::Error + Send + Sync + 'static>,
{
	let Some(value) = env::var(var).ok() else {
		return Ok(None);
	};

	if value.is_empty() {
		return Ok(None);
	}

	value
		.parse::<T>()
		.map(Some)
		.map_err(|error| InitializeConfigError::Parse { var, source: Box::new(error) })
}
