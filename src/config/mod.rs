use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use smart_debug::SmartDebug;
use url::Url;

mod error;
pub use error::{Error, Result};

pub mod axiom;
pub mod database;
pub mod jwt;
pub mod steam;

/// The API configuration.
#[derive(SmartDebug)]
pub struct Config {
	/// Address to open a TCP socket on.
	#[debug("`{}`")]
	pub socket_addr: SocketAddrV4,

	/// The public URL of the API.
	#[debug("`{}`")]
	pub public_url: Url,

	/// Wildcard `Domain` field for HTTP cookies.
	#[debug("`{}`")]
	pub domain: String,

	/// The API's database configuration.
	pub database: database::Config,

	/// The API's axiom configuration.
	pub axiom: Option<axiom::Config>,

	/// The API's JWT configuration.
	pub jwt: jwt::Config,

	/// The API's Steam configuration.
	pub steam: steam::Config,
}

impl Config {
	/// Creates a new [Config] instance by parsing relevant environment variables.
	pub fn new() -> Result<Self> {
		let ip_addr = get_env_var::<Ipv4Addr>("KZ_API_IP")?;
		let port = get_env_var::<u16>("KZ_API_PORT")?;
		let socket_addr = SocketAddrV4::new(ip_addr, port);
		let public_url = get_env_var::<Url>("KZ_API_URL")?;

		let domain = match public_url.domain() {
			Some(domain) => domain_to_wildcard(domain).to_owned(),
			None => public_url
				.host_str()
				.map(ToOwned::to_owned)
				.expect("API url should have a host"),
		};

		let database = database::Config::new()?;
		let axiom = axiom::Config::new().ok();
		let jwt = jwt::Config::new()?;
		let steam = steam::Config::new()?;

		Ok(Self { socket_addr, public_url, domain, database, axiom, jwt, steam })
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

/// Converts a domain to a wildcard domain for cookies.
/// Subdomains are simply cut off.
fn domain_to_wildcard(mut domain: &str) -> &str {
	let mut first_period = None;
	let mut total_periods = 0;

	for (idx, char) in domain.chars().enumerate() {
		if char != '.' {
			continue;
		}

		total_periods += 1;

		if first_period.is_none() {
			first_period = Some(idx);
		}
	}

	if total_periods > 1 {
		domain = &domain[first_period.unwrap()..];
	}

	domain
}
