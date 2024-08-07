//! This module contains the [`Config`] struct - a set of configuration options
//! that will be read from a file on startup.
//!
//! See the `.config/config.example.toml` file in the root of the repository for
//! all the available options and their default values.

#![allow(clippy::disallowed_types)]

use std::net::{IpAddr, SocketAddr};
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::{Deserialize, Deserializer};
use thiserror::Error;
use tracing_subscriber::EnvFilter;
use url::Url;

/// The API's runtime configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Tokio configuration.
	pub runtime: RuntimeConfig,

	/// Tracing configuration.
	pub tracing: TracingConfig,

	/// Database configuration.
	pub database: DatabaseConfig,

	/// HTTP configuration.
	pub http: HttpConfig,

	/// Secrets.
	pub secrets: Secrets,

	/// Steam configuration.
	pub steam: SteamConfig,
}

impl Config
{
	/// Loads a configuration file located at `path` from disk and parses it
	/// into a [`Config`].
	pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadConfigError>
	{
		let file = fs::read_to_string(path).map_err(LoadConfigError::ReadFile)?;
		let config = toml::from_str(&file).map_err(LoadConfigError::ParseFile)?;

		Ok(config)
	}
}

/// Tokio configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuntimeConfig
{
	/// The amount of worker threads to spawn.
	#[serde(deserialize_with = "deserialize_zero_as_none_usize")]
	pub worker_threads: Option<NonZero<usize>>,

	/// The maximum amount of blocking threads to spawn.
	#[serde(deserialize_with = "deserialize_zero_as_none_usize")]
	pub max_blocking_threads: Option<NonZero<usize>>,

	/// The stack size (in bytes) for any spawned threads.
	pub thread_stack_size: usize,

	/// Tokio tracing configuration.
	#[cfg(feature = "console")]
	pub metrics: RuntimeMetricsConfig,
}

/// Tokio tracing configuration.
#[cfg(feature = "console")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuntimeMetricsConfig
{
	/// Record task poll times.
	pub record_poll_counts: bool,
}

/// Tracing configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TracingConfig
{
	/// Enable tracing.
	pub enable: bool,

	/// The default global filter.
	#[serde(deserialize_with = "deserialize_env_filter")]
	pub filter: EnvFilter,

	/// Configuration for writing trace data to stderr.
	pub stderr: TracingStderrConfig,

	/// Configuration for writing trace data to files.
	pub files: TracingFilesConfig,

	/// Configuration for collecting trace data with tokio-console.
	#[cfg(feature = "console")]
	pub console: TracingConsoleConfig,
}

/// Configuration for writing trace data to stderr.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TracingStderrConfig
{
	/// Write trace data to stderr.
	pub enable: bool,

	/// Emit ANSI escape codes for formatting (colors, italics, etc.).
	pub ansi: bool,

	/// Additional filter directives for this layer.
	#[serde(deserialize_with = "deserialize_env_filter_opt")]
	pub filter: Option<EnvFilter>,
}

/// Configuration for writing trace data to files.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TracingFilesConfig
{
	/// Path to the directory to store logs in.
	pub path: PathBuf,

	/// Additional filter directives for this layer.
	#[serde(deserialize_with = "deserialize_env_filter_opt")]
	pub filter: Option<EnvFilter>,
}

/// Configuration for collecting trace data with tokio-console.
#[cfg(feature = "console")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TracingConsoleConfig
{
	/// Address to listen for client connections on.
	pub server_addr: Option<TokioConsoleServerAddr>,
}

/// Server address for tokio-console to listen on.
#[cfg(feature = "console")]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokioConsoleServerAddr
{
	/// A TCP address.
	Tcp(SocketAddr),

	/// A Unix Domain Socket path.
	Unix(PathBuf),
}

/// Database configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DatabaseConfig
{
	/// Connection URL
	#[serde(skip_deserializing, default = "database_url")]
	pub url: Url,

	/// Minimum amount of pool connections to open right away.
	pub min_connections: u32,

	/// Maximum amount of pool connections to open right away.
	#[serde(deserialize_with = "deserialize_zero_as_none_u32")]
	pub max_connections: Option<NonZero<u32>>,
}

/// HTTP configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpConfig
{
	/// IP address to listen on.
	pub listen_addr: IpAddr,

	/// Port to listen on.
	pub listen_port: u16,

	/// The URL that other services can use to reach the API.
	pub public_url: Url,

	/// The value to use for `Domain` fields in HTTP cookies.
	pub cookie_domain: String,
}

impl HttpConfig
{
	/// Returns a full [`SocketAddr`] composed of the values stored in this
	/// struct.
	pub fn socket_addr(&self) -> SocketAddr
	{
		SocketAddr::new(self.listen_addr, self.listen_port)
	}
}

/// Secrets.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Secrets
{
	/// Key to use for encoding/decoding JWTs.
	pub jwt_key: String,
}

/// Steam configuration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SteamConfig
{
	/// Steam WebAPI key.
	pub api_key: String,

	/// Path to use for storing downloaded workshop assets.
	pub workshop_artifacts_path: PathBuf,

	/// Path to the `DepotDownloader` executable.
	pub depot_downloader_path: PathBuf,
}

/// Errors that can occur when loading a config file.
#[derive(Debug, Error)]
pub enum LoadConfigError
{
	/// The file could not be read from disk.
	#[error("failed to read config file: {0}")]
	ReadFile(io::Error),

	/// The file could not be parsed.
	#[error("failed to parse config file: {0}")]
	ParseFile(toml::de::Error),
}

/// Deserializes a [`NonZero<32>`] and turns 0 into [`None`].
fn deserialize_zero_as_none_u32<'de, D>(deserializer: D) -> Result<Option<NonZero<u32>>, D::Error>
where
	D: Deserializer<'de>,
{
	u32::deserialize(deserializer).map(NonZero::new)
}

/// Deserializes a [`NonZero<usize>`] and turns 0 into [`None`].
fn deserialize_zero_as_none_usize<'de, D>(
	deserializer: D,
) -> Result<Option<NonZero<usize>>, D::Error>
where
	D: Deserializer<'de>,
{
	usize::deserialize(deserializer).map(NonZero::new)
}

/// Loads `DATABASE_URL` from the environment.
///
/// # Panics
///
/// This function will panic if `DATABASE_URL` is not set or not a valid URL.
fn database_url() -> Url
{
	env::var("DATABASE_URL")
		.expect("`DATABASE_URL` should be set")
		.parse()
		.expect("`DATABASE_URL` must be a valid URL")
}

/// Deserializes [`EnvFilter`] directives.
fn deserialize_env_filter<'de, D>(deserializer: D) -> Result<EnvFilter, D::Error>
where
	D: Deserializer<'de>,
{
	String::deserialize(deserializer)?
		.parse::<EnvFilter>()
		.map_err(serde::de::Error::custom)
}

/// Deserializes optional [`EnvFilter`] directives and treats an empty string as
/// [`None`].
fn deserialize_env_filter_opt<'de, D>(deserializer: D) -> Result<Option<EnvFilter>, D::Error>
where
	D: Deserializer<'de>,
{
	Option::<String>::deserialize(deserializer)?
		.filter(|s| !s.is_empty())
		.map(|directives| directives.parse::<EnvFilter>())
		.transpose()
		.map_err(serde::de::Error::custom)
}
