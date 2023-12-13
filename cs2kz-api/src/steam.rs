//! This module holds structs specific to communication with the Steam API.

use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

static CALLBACK_ROUTE: &str = "/auth/steam/callback";

/// This is the data we send to Steam when redirecting a user to login.
#[derive(Debug, Serialize)]
pub struct RedirectForm {
	#[serde(rename = "openid.ns")]
	ns: &'static str,

	#[serde(rename = "openid.identity")]
	identity: &'static str,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: &'static str,

	#[serde(rename = "openid.mode")]
	mode: &'static str,

	#[serde(rename = "openid.realm")]
	callback_host: Url,

	#[serde(rename = "openid.return_to")]
	callback_url: Url,
}

impl RedirectForm {
	/// Constructs a new [`RedirectForm`].
	pub fn new(callback_host: Url) -> Self {
		let callback_url = callback_host
			.join(CALLBACK_ROUTE)
			.expect("this is a valid URL");

		Self {
			ns: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			callback_host,
			callback_url,
		}
	}
}

/// This is what Steam sends after redirecting a user back to the API after their login.
///
/// There are more fields here, technically, but we don't care about them.
#[derive(Debug, Deserialize)]
pub struct AuthResponse {
	/// The user's SteamID.
	#[serde(rename = "openid.claimed_id", deserialize_with = "deser_steam_id")]
	pub steam_id: SteamID,
}

fn deser_steam_id<'de, D>(deserializer: D) -> Result<SteamID, D::Error>
where
	D: Deserializer<'de>,
{
	use serde::de::{Error as E, Unexpected as U};

	let url = Url::deserialize(deserializer)?;
	let steam_id = url
		.path_segments()
		.ok_or(E::custom("missing SteamID from path"))?
		.last()
		.ok_or(E::custom("missing SteamID from path"))?;

	let steam_id = steam_id
		.parse::<u64>()
		.map_err(|_| E::invalid_type(U::Str(steam_id), &"64-bit SteamID"))?;

	SteamID::from_u64(steam_id)
		.map_err(|_| E::invalid_value(U::Unsigned(steam_id), &"64-bit SteamID"))
}
