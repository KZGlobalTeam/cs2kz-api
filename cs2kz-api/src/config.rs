use {
	color_eyre::eyre::Context,
	serde::Deserialize,
	std::{
		net::{IpAddr, Ipv4Addr, SocketAddr},
		path::Path,
	},
};

/// The configuration for the API.
///
/// An instance of this struct will be built from a config file that will be read at startup.
/// An example file is available in `@/configs/api.example.toml`.
///
/// Any values here can be overridden via CLI arguments.
#[derive(Debug, Deserialize)]
pub struct Config {
	/// The IP address the API will run on.
	///
	/// This will default to `127.0.0.1` (localhost).
	#[serde(default = "Config::default_address")]
	pub address: Ipv4Addr,

	/// The port the API will be exposed on.
	#[serde(default = "Config::default_port")]
	pub port: u16,

	/// Whether to enable logging.
	/// By default logging is enabled.
	#[serde(default = "Config::default_logging")]
	pub enable_logging: bool,

	/// MySQL connection string.
	#[serde(default = "Config::default_database_url")]
	pub database_url: String,
}

impl Config {
	/// The default IP address for the API.
	const fn default_address() -> Ipv4Addr {
		Ipv4Addr::new(127, 0, 0, 1)
	}

	/// The default port for the API.
	const fn default_port() -> u16 {
		8069
	}

	/// The default value for enabling logging.
	const fn default_logging() -> bool {
		true
	}

	fn default_database_url() -> String {
		std::env::var("DATABASE_URL")
			.expect("Missing `DATABASE_URL` environment variable or configuration option.")
	}

	/// Creates a [`SocketAddr`] from the specified IP address and port.
	pub const fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(IpAddr::V4(self.address), self.port)
	}

	/// Will attempt to read a config file from the specified `path` and parse it into
	/// [`Config`].
	pub fn from_path(path: &Path) -> color_eyre::Result<Config> {
		let file = std::fs::read_to_string(path).context("Failed to read config file.")?;
		let config: Config = serde_toml::from_str(&file).context("Failed to parse config file.")?;

		Ok(config)
	}
}
