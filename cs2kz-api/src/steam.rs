use color_eyre::eyre::Context;
use color_eyre::Result;
use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

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
	pub fn new(callback_host: Url, callback_route: &str) -> Result<Self> {
		let callback_url = callback_host
			.join(callback_route)
			.context("Invalid callback route.")?;

		Ok(Self {
			ns: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			callback_host,
			callback_url,
		})
	}
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
	#[serde(
		rename = "openid.claimed_id",
		deserialize_with = "deserialize_steam_id"
	)]
	pub steam_id: SteamID,
}

fn deserialize_steam_id<'de, D>(deserializer: D) -> Result<SteamID, D::Error>
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

	SteamID::from_id64(steam_id)
		.map_err(|_| E::invalid_value(U::Unsigned(steam_id), &"64-bit SteamID"))
}
