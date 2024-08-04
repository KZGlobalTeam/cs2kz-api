//! CS2 server hosts.
//!
//! CS2 servers are allowed to use both IPv4/IPv6 and full domain names. This
//! module defines a `Host` type that encapsulates any of these 3, and can
//! encode/decode them properly.

use std::convert::Infallible;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::net::IpAddr;

/// A CS2 server host.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum Host
{
	/// An IP address.
	#[schema(title = "Ip")]
	Ip(IpAddr),

	/// A domain.
	#[schema(title = "Domain")]
	Domain(String),
}

impl FromStr for Host
{
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(ip) = value.parse::<IpAddr>() {
			Ok(Self::Ip(ip))
		} else {
			Ok(Self::Domain(value.to_owned()))
		}
	}
}

impl From<String> for Host
{
	fn from(value: String) -> Self
	{
		if let Ok(ip) = value.parse::<IpAddr>() {
			Self::Ip(ip)
		} else {
			Self::Domain(value)
		}
	}
}

crate::macros::sqlx_scalar_forward!(Host as String => {
	encode: |self| {
		match self {
			Host::Ip(ip) => ip.to_string(),
			Host::Domain(domain) => domain.clone(),
		}
	},

	decode: |value| { value.parse()? },
});
