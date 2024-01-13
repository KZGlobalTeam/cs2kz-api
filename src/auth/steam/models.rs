use std::sync::Arc;

use cs2kz::SteamID;
use reqwest::header;
use serde::{Deserialize, Serialize};
use tracing::trace;
use url::Url;
use utoipa::IntoParams;

use crate::{Error, Result};

/// Payload to be sent to Steam when redirecting a user for login.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct AuthRequest {
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

impl AuthRequest {
	pub const VERIFY_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Extracts the SteamID from this request.
	pub fn steam_id(&self) -> Option<SteamID> {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|steam_id| steam_id.parse().ok())
	}

	/// Ensures this request can be trusted by sending it to the Steam API and verifying the
	/// result for validity.
	///
	/// On success it will return the SteamID of the player who logged in, as well as the
	/// original URL they came from.
	pub async fn validate(
		mut self,
		public_url: &Url,
		http_client: Arc<reqwest::Client>,
	) -> Result<(SteamID, Url)> {
		if self.return_to.host() != public_url.host() {
			trace!(%self.return_to, public_host = ?public_url.host(), "invalid return URL");
			return Err(Error::ForeignHost);
		}

		self.mode = String::from("check_authentication");
		let query = serde_urlencoded::to_string(&self).expect("this is valid");

		let is_valid = http_client
			.post(Self::VERIFY_URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(query)
			.send()
			.await
			.and_then(|res| res.error_for_status())
			.map_err(|err| {
				trace!(%err, "failed to authenticate user");
				Error::Unauthorized
			})?
			.text()
			.await
			.map_err(|err| {
				trace!(%err, "steam response was not text");
				crate::steam::Error::Http(err)
			})?
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

		trace!(%steam_id, origin = %self.origin_url, "user logged in with steam");

		Ok((steam_id, self.origin_url))
	}
}
