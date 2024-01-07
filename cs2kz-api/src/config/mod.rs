//! This module holds a [`Config`] struct for the API.
//!
//! These values are read once on startup.

use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::result::Result as StdResult;
use std::str::FromStr;

use thiserror::Error as ThisError;
use url::Url;

mod error;
pub use error::{Error, Result};

/// CS2KZ API configuration.
///
/// This will be built from environment variables at runtime and passed around the application to
/// control various things.
#[derive(Debug)]
pub struct Config {
	/// The private IP address and port the API will be exposed on.
	pub socket_addr: SocketAddrV4,

	/// The public URL the API will be accessible from.
	pub api_url: Url,

	/// Connection string for the API's database.
	pub database_url: Url,

	/// Secret key for encoding and decoding JWTs.
	pub jwt_secret: String,

	/// The environment the API is running in.
	pub environment: Environment,

	/// The name of the dataset to send the logs to.
	pub axiom_dataset: String,

	/// [Steam WebAPI] Key.
	///
	/// [Steam WebAPI]: https://steamcommunity.com/dev
	pub steam_api_key: String,
}

impl Config {
	/// Loads all necessary environment variables to build the API's configuration.
	pub async fn new() -> Result<Self> {
		let ip = Self::load_var::<Ipv4Addr>("KZ_API_IP")?;
		let port = Self::load_var::<u16>("KZ_API_PORT")?;
		let socket_addr = SocketAddrV4::new(ip, port);
		let api_url = Self::load_var::<Url>("KZ_API_URL")?;
		let database_url = Self::load_var::<Url>("DATABASE_URL")?;
		let jwt_secret = Self::load_var::<String>("KZ_API_JWT_SECRET")?;
		let environment = Self::load_var::<Environment>("KZ_API_ENVIRONMENT")?;
		let axiom_dataset = Self::load_var::<String>("AXIOM_DATASET")?;
		let steam_api_key = Self::load_var::<String>("STEAM_API_KEY")?;

		Ok(Self {
			socket_addr,
			api_url,
			database_url,
			jwt_secret,
			environment,
			axiom_dataset,
			steam_api_key,
		})
	}

	/// Loads the given `variable` from the environment and parses it.
	pub(crate) fn load_var<T>(variable: &'static str) -> Result<T>
	where
		T: FromStr,
	{
		env::var(variable)
			.map_err(|_| Error::MissingConfigVariable { variable })?
			.parse::<T>()
			.map_err(|_| Error::InvalidConfigVariable {
				variable,
				expected: std::any::type_name::<T>(),
			})
	}
}

/// The environment the API is running in.
///
/// This may change runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
	/// The API is running locally on the developer's computer.
	Development,

	/// The API is running in production.
	Production,
}

/// A string could not be parsed into a valid [`Environment`].
#[derive(Debug, ThisError)]
#[error("`{0}` is not a valid environment to run in. Expected one of: [development, production]")]
pub struct InvalidEnvironment(String);

impl FromStr for Environment {
	type Err = InvalidEnvironment;

	fn from_str(s: &str) -> StdResult<Self, Self::Err> {
		match s {
			"development" => Ok(Self::Development),
			"production" => Ok(Self::Production),
			s => Err(InvalidEnvironment(s.to_owned())),
		}
	}
}
