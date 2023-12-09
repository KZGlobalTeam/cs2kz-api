use std::net::{IpAddr, SocketAddr};

use color_eyre::eyre::Context;
use url::Url;

/// The configuration for the API.
///
/// An instance of this struct will be built from environment variables stored in a `.env` file.
#[derive(Debug)]
pub struct Config {
	/// The internal address to expose the API on.
	pub socket_addr: SocketAddr,

	/// The public URL of the API.
	pub public_url: Url,

	/// MySQL connection string.
	pub database_url: String,

	/// JWT secret key for encoding / decoding authentication headers.
	pub jwt_secret: String,
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
	/// Loads config values from the environment.
	pub fn load() -> color_eyre::Result<Self> {
		let ip_address = load_env!("API_IP");
		let port = load_env!("API_PORT");
		let socket_addr = SocketAddr::new(IpAddr::V4(ip_address), port);
		let public_url = load_env!("API_PUBLIC_URL");
		let database_url = load_env!("DATABASE_URL");
		let jwt_secret = load_env!("JWT_SECRET");

		Ok(Self { socket_addr, public_url, database_url, jwt_secret })
	}
}
