//! This module holds structs specific to communication with the Steam API.

use cs2kz::SteamID;
use reqwest::header;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use tracing::trace;
use url::Url;

use crate::{Error, Result};

static CALLBACK_ROUTE: &str = "/auth/steam/callback";
static STEAM_LOGIN_VERIFY_URL: &str = "https://steamcommunity.com/openid/login";

/// This is the data we send to Steam when redirecting a user to login.
#[derive(Debug, Clone, Serialize)]
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
	pub(crate) callback_url: Url,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
	/// The API's domain, if valid.
	#[serde(rename = "openid.return_to")]
	pub return_to: Url,

	/// The original URL this request came from.
	#[serde(skip_serializing)]
	origin_url: Url,

	#[serde(rename = "openid.mode")]
	mode: String,

	#[serde(rename = "openid.ns")]
	ns: String,

	#[serde(rename = "openid.op_endpoint")]
	op_endpoint: String,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: Url,

	#[serde(rename = "openid.identity")]
	identity: Option<String>,

	#[serde(rename = "openid.response_nonce")]
	response_nonce: String,

	#[serde(rename = "openid.invalidate_handle")]
	invalidate_handle: Option<String>,

	#[serde(rename = "openid.assoc_handle")]
	assoc_handle: String,

	#[serde(rename = "openid.signed")]
	signed: String,

	#[serde(rename = "openid.sig")]
	sig: String,
}

impl AuthResponse {
	/// Extracts the claimed SteamID from the request body.
	pub fn steam_id(&self) -> Option<SteamID> {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|steam_id| steam_id.parse().ok())
	}

	/// Validates this response with Steam's API and extracts the claimed SteamID
	/// and original request URL.
	pub async fn validate(
		mut self,
		public_url: &Url,
		http_client: &reqwest::Client,
	) -> Result<(SteamID, Url)> {
		if self.return_to.host() != public_url.host() {
			trace!(%self.return_to, "invalid return URL");
			return Err(Error::Unauthorized);
		}

		self.mode = String::from("check_authentication");
		let query = serde_urlencoded::to_string(&self).expect("this is valid");

		let response = http_client
			.post(STEAM_LOGIN_VERIFY_URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(query)
			.send()
			.await
			.and_then(|res| res.error_for_status())
			.map_err(|err| {
				trace!(?err, "failed to authenticate user");
				Error::Unauthorized
			})?;

		let body = response.text().await.map_err(|err| {
			trace!(?err, "steam response was not text");
			Error::Unauthorized
		})?;

		let is_valid = body
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_some();

		if !is_valid {
			trace!("request was invalid");
			return Err(Error::Unauthorized);
		}

		let steam_id = self.steam_id().ok_or_else(|| {
			trace!("steam response did not include a SteamID");
			Error::Unauthorized
		})?;

		trace!(%steam_id, %self.origin_url, "user logged in with steam");

		Ok((steam_id, self.origin_url))
	}
}

/// A Steam Workshop map.
#[derive(Debug)]
pub struct WorkshopMap {
	/// The map's name.
	pub name: String,

	/// The map's filesize.
	pub filesize: u64,
}

#[derive(Debug)]
struct GetWorkshopMapParams(u32);

impl WorkshopMap {
	const URL: &'static str =
		"https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

	/// Fetches the workshop map with the given `id` from Steam.
	pub async fn get(id: u32, http_client: &reqwest::Client) -> Option<Self> {
		let params = GetWorkshopMapParams(id);
		let params = serde_urlencoded::to_string(&params).expect("this is valid");

		let workshop_map = http_client
			.post(Self::URL)
			.header("Content-Type", "application/x-www-form-urlencoded")
			.body(params)
			.send()
			.await
			.ok()?
			.json()
			.await
			.ok()?;

		Some(workshop_map)
	}
}

impl<'de> Deserialize<'de> for WorkshopMap {
	fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error as E;

		let object = JsonValue::deserialize(deserializer)
			.map(|mut object| object["response"].take())
			.map(|mut object| object["publishedfiledetails"].take())
			.map(|mut object| object[0].take())?;

		let name = object["title"]
			.as_str()
			.ok_or_else(|| E::missing_field("title"))?
			.to_owned();

		let filesize = object["file_size"]
			.as_u64()
			.ok_or_else(|| E::missing_field("file_size"))?;

		Ok(Self { name, filesize })
	}
}

impl Serialize for GetWorkshopMapParams {
	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_struct("params", 2)?;

		serializer.serialize_field("itemcount", &1)?;
		serializer.serialize_field("publishedfileids[0]", &self.0)?;
		serializer.end()
	}
}

#[cfg(test)]
mod tests {
	use color_eyre::eyre::ContextCompat;
	use color_eyre::Result;

	use super::WorkshopMap;

	#[tokio::test]
	async fn fetch_map() -> Result<()> {
		let client = reqwest::Client::new();
		let id = 3121168339;
		let map = WorkshopMap::get(id, &client)
			.await
			.context("Failed to fetch Workshop map.")?;

		assert_eq!(map.name, "kz_grotto");
		assert!(map.filesize > 0);

		Ok(())
	}
}
