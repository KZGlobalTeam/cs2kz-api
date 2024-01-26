use std::sync::Arc;

use axum::async_trait;
use axum::extract::{FromRequestParts, Query};
use axum::http::request;
use axum::response::Redirect;
use cs2kz::SteamID;
use reqwest::header;
use serde::{Deserialize, Serialize};
use tracing::trace;
use url::Url;
use utoipa::IntoParams;

use crate::auth::services::models::ServiceKey;
use crate::{Error, Result};

/// Form to send to steam when redirecting a user for login.
#[derive(Debug, Clone, Serialize)]
pub struct LoginForm {
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

impl LoginForm {
	pub const CALLBACK_ROUTE: &'static str = "/auth/steam/callback";
	pub const REDIRECT_URL: &'static str = "https://steamcommunity.com/openid/login";

	pub fn new(callback_host: Url) -> Self {
		let callback_url = callback_host
			.join(Self::CALLBACK_ROUTE)
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

	/// Creates a [Redirect] to Steam, which will redirect back to the given `origin_url` after
	/// a successful login.
	pub fn with_info(mut self, origin_url: Url, service_key: ServiceKey) -> Redirect {
		self.callback_url
			.query_pairs_mut()
			.append_pair("origin_url", origin_url.as_str())
			.append_pair("service_key", &service_key.to_string());

		let query = serde_urlencoded::to_string(&self).expect("this is a valid query string");
		let mut url = Url::parse(Self::REDIRECT_URL).expect("this is a valid url");

		url.set_query(Some(&query));

		Redirect::to(url.as_str())
	}
}

/// Payload to be sent to Steam when redirecting a user for login.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct Auth {
	/// The API's domain, if valid.
	#[serde(rename = "openid.return_to")]
	pub return_to: Url,

	/// The original URL this request came from.
	#[serde(skip_serializing)]
	pub origin_url: Url,

	/// The service key of the service this request originally came from.
	#[serde(skip_serializing)]
	pub service_key: ServiceKey,

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

impl Auth {
	pub const VERIFY_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Extracts the SteamID from this request.
	///
	/// # Panics
	///
	/// This method will panic if `self` does not contain a valid SteamID.
	/// For validated requests this should never happen, and is probably a bug if it does.
	pub fn steam_id(&self) -> SteamID {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|steam_id| steam_id.parse().ok())
			.expect("steam auth request did not have SteamID")
	}
}

#[async_trait]
impl FromRequestParts<Arc<crate::State>> for Auth {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &Arc<crate::State>,
	) -> Result<Self> {
		let Query(mut auth) = Query::<Self>::from_request_parts(parts, state)
			.await
			.map_err(|err| {
				trace!(%err, "invalid query params");
				Error::Unauthorized
			})?;

		let config = state.config();
		let public_url = &config.public_url;

		if auth.return_to.host() != public_url.host() {
			trace!(%auth.return_to, public_host = ?public_url.host(), "invalid return URL");
			return Err(Error::ForeignHost);
		}

		auth.mode = String::from("check_authentication");
		let query = serde_urlencoded::to_string(&auth).expect("this is valid");

		let is_valid = state
			.http()
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

		let steam_id = auth.steam_id();

		trace!(%steam_id, origin = %auth.origin_url, "user logged in with steam");

		Ok(auth)
	}
}
