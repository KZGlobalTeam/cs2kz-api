use std::fmt::{self, Debug};
use std::net::{Ipv4Addr, SocketAddrV4};

use url::Url;

mod error;
pub use error::{Error, Result};

pub mod database;
pub mod jwt;
pub mod steam;

/// The API configuration.
pub struct Config {
	/// Address to open a TCP socket on.
	pub socket_addr: SocketAddrV4,

	/// The public URL of the API.
	pub public_url: Url,

	/// Wildcard `Domain` field for HTTP cookies.
	pub domain: String,

	/// The API's database configuration.
	pub database: database::Config,

	/// The API's JWT configuration.
	pub jwt: jwt::Config,

	/// The API's Steam configuration.
	pub steam: steam::Config,
}

impl Config {
	/// Creates a new [Config] instance by parsing relevant environment variables.
	///
	/// # Panics
	///
	/// This function will panic if the `KZ_API_URL` environment variable has an unexpected
	/// shape.
	pub fn new() -> Result<Self> {
		let ip_addr = crate::env::get::<Ipv4Addr>("API_IP")?;
		let port = crate::env::get::<u16>("API_PORT")?;
		let socket_addr = SocketAddrV4::new(ip_addr, port);
		let public_url = crate::env::get::<Url>("API_URL")?;

		let domain = public_url
			.domain()
			.map(|domain| domain_to_wildcard(domain).to_owned())
			.unwrap_or_else(|| {
				public_url
					.host_str()
					.map(ToOwned::to_owned)
					.expect("API url should have a host")
			});

		let database = database::Config::new()?;
		let jwt = jwt::Config::new()?;
		let steam = steam::Config::new()?;

		Ok(Self { socket_addr, public_url, domain, database, jwt, steam })
	}
}

/// Converts a domain to a wildcard domain for cookies.
/// Subdomains are simply cut off.
fn domain_to_wildcard(mut domain: &str) -> &str {
	/// State Machine to keep track of `.` count
	enum State {
		/// We have seen none so far
		None,

		/// We have seen one; at byte index `idx`
		One { idx: usize },

		/// We have seen more than one; the first one was at byte index `idx`
		Many { first_idx: usize },
	}

	let final_state = domain
		.char_indices()
		.fold(State::None, |state, (idx, char)| match (char, state) {
			('.', State::None) => State::One { idx },
			('.', State::One { idx }) => State::Many { first_idx: idx },
			(_, state) => state,
		});

	if let State::Many { first_idx } = final_state {
		domain = &domain[first_idx..];
	}

	domain
}

impl Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Config")
			.field("socket_addr", &format_args!("{}", self.socket_addr))
			.field("public_url", &self.public_url.as_str())
			.field("domain", &self.domain)
			.field("database", &self.database)
			.field("jwt", &self.jwt)
			.field("steam", &self.steam)
			.finish()
	}
}
