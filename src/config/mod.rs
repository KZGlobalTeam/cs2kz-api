use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::{env, fmt};

use url::Url;

mod error;
pub use error::{Error, Result};

mod environment;
pub use environment::Environment;

pub mod axiom;
pub mod database;
pub mod jwt;
pub mod steam;

/// The API configuration.
pub struct Config {
	/// Address to open a TCP socket on.
	pub(crate) socket_addr: SocketAddrV4,

	/// The public URL of the API.
	pub(crate) public_url: Url,

	/// The environment the API is currently running in.
	pub(crate) environment: Environment,

	/// The API's database configuration.
	pub(crate) database: database::Config,

	/// The API's axiom configuration.
	pub(crate) axiom: Option<axiom::Config>,

	/// The API's JWT configuration.
	pub(crate) jwt: jwt::Config,

	/// The API's Steam configuration.
	pub(crate) steam: steam::Config,
}

impl Config {
	/// Creates a new [Config] instance by parsing relevant environment variables.
	pub fn new() -> Result<Self> {
		let ip_addr = get_env_var::<Ipv4Addr>("KZ_API_IP")?;
		let port = get_env_var::<u16>("KZ_API_PORT")?;
		let socket_addr = SocketAddrV4::new(ip_addr, port);
		let public_url = get_env_var("KZ_API_URL")?;
		let environment = get_env_var("KZ_API_ENV")?;
		let database = database::Config::new()?;
		let axiom = axiom::Config::new().ok();
		let jwt = jwt::Config::new()?;
		let steam = steam::Config::new()?;

		Ok(Self { socket_addr, public_url, environment, database, axiom, jwt, steam })
	}

	pub const fn socket_addr(&self) -> SocketAddr {
		SocketAddr::V4(self.socket_addr)
	}

	pub const fn database(&self) -> &database::Config {
		&self.database
	}

	pub const fn axiom(&self) -> Option<&axiom::Config> {
		self.axiom.as_ref()
	}
}

fn get_env_var<T>(var: &'static str) -> Result<T>
where
	T: FromStr,
	<T as FromStr>::Err: Into<Error>,
{
	env::var(var)
		.map_err(|_| Error::MissingEnvironmentVariable(var))
		.and_then(|var| var.parse().map_err(Into::into))
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("CS2KZ API Config")
			.field("socket_addr", &format_args!("{}", self.socket_addr))
			.field("public_url", &format_args!("{}", self.public_url))
			.field("environment", &self.environment)
			.field("database", &self.database)
			.field("axiom", &self.axiom)
			.field("jwt", &self.jwt)
			.field("steam", &self.steam)
			.finish()
	}
}
