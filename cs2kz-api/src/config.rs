use {
	color_eyre::eyre::Context,
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
};

/// The configuration for the API.
///
/// An instance of this struct will be built from environment variables stored in a `.env` file.
#[derive(Debug)]
pub struct Config {
	/// The IP address the API will run on.
	pub ip_address: Ipv4Addr,

	/// The port the API will be exposed on.
	pub port: u16,

	/// Whether to enable logging.
	pub enable_logging: bool,

	/// MySQL connection string.
	pub database_url: String,
}

macro_rules! load_env {
	($var:literal) => {
		std::env::var($var)
			.context(concat!("Missing `", $var, "` environment variable."))?
			.parse()
			.context(concat!("Invalid `", $var, "` environment variable."))?
	};
}

impl Config {
	/// Creates a [`SocketAddr`] from the specified IP address and port.
	pub const fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(IpAddr::V4(self.ip_address), self.port)
	}

	/// Loads config values from the environment.
	pub fn load() -> color_eyre::Result<Self> {
		let ip_address = load_env!("API_IP");
		let port = load_env!("API_PORT");
		let enable_logging = load_env!("API_LOGGING");
		let database_url = load_env!("DATABASE_URL");

		Ok(Self { ip_address, port, enable_logging, database_url })
	}
}
