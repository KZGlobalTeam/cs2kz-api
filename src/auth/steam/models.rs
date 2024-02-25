use axum::async_trait;
use axum::extract::{FromRequestParts, Query};
use axum::http::request;
use axum::response::Redirect;
use cs2kz::SteamID;
use reqwest::header;
use serde::{Deserialize, Serialize};
use tracing::{trace, warn};
use url::Url;
use utoipa::IntoParams;

use crate::{Error, Result, State};

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
	/// API route that Steam is supposed to redirect back to after the login process is
	/// complete.
	pub const CALLBACK_ROUTE: &'static str = "/auth/steam/callback";

	/// Steam API URL to redirect a user to so they can login.
	pub const REDIRECT_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Constructs a new [`LoginForm`].
	///
	/// # Panics
	///
	/// This function will panic if there is a bug in an internal serialization implementation.
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
	///
	/// # Panics
	///
	/// This function will panic if there is a bug in an internal serialization implementation.
	pub fn with_origin_url(mut self, origin_url: &Url) -> Redirect {
		self.callback_url
			.query_pairs_mut()
			.append_pair("origin_url", origin_url.as_str());

		let query = serde_urlencoded::to_string(&self).expect("this is a valid query string");
		let mut url = Url::parse(Self::REDIRECT_URL).expect("this is a valid url");

		url.set_query(Some(&query));

		Redirect::to(url.as_str())
	}
}

/// Payload to be sent to Steam when redirecting a user for login.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct Auth {
	/// The original URL this request came from.
	#[serde(skip_serializing)]
	pub origin_url: Url,

	#[serde(rename = "openid.return_to")]
	return_to: Url,

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
	/// Steam API URL for verifying OpenID login requests.
	pub const VERIFY_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Extracts the SteamID from this request.
	///
	/// # Panics
	///
	/// This method will panic if `self` does not contain a valid SteamID.
	/// For validated requests this should never happen, and so is probably a bug if it does.
	pub fn steam_id(&self) -> SteamID {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|steam_id| steam_id.parse().ok())
			.expect("steam auth request did not have SteamID")
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for Auth {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let Query(mut auth) = Query::<Self>::from_request_parts(parts, state).await?;

		let api_host = &state.config.domain;

		let Some(origin_host) = auth.origin_url.host_str() else {
			trace!(%auth.origin_url, "origin has no host");
			return Err(Error::invalid("`origin_url`")
				.with_detail(format_args!("got: `{}`", auth.origin_url)));
		};

		let mut is_known_host = origin_host.ends_with(api_host);

		if !is_known_host && cfg!(not(feature = "production")) {
			warn!(%origin_host, %api_host, "allowing mismatching hosts due to dev mode");
			is_known_host = true;
		}

		if !is_known_host {
			trace!(%origin_host, %api_host, "rejecting login due to mismatching hosts");
			return Err(Error::invalid("origin host")
				.with_detail(format_args!("expected host to match `{api_host}`")));
		}

		auth.mode = String::from("check_authentication");
		let query = serde_urlencoded::to_string(&auth).expect("this is valid");

		let is_valid = state
			.http_client
			.post(Self::VERIFY_URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(query)
			.send()
			.await
			.and_then(|res| res.error_for_status())?
			.text()
			.await?
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_some();

		if !is_valid {
			trace!("request was invalid");
			return Err(Error::invalid("query parameters")
				.with_detail("request was not issued by steam")
				.unauthorized());
		}

		let steam_id = auth.steam_id();

		trace!(%steam_id, origin = %auth.origin_url, "user logged in with steam");

		Ok(auth)
	}
}
