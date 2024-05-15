//! Authentication with the Steam API via OpenID.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum::response::Redirect;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use reqwest::{header, Response};
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;
use utoipa::IntoParams;

use crate::{Error, Result, State};

/// Form used for logging a user into Steam.
#[derive(Debug, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct LoginForm {
	#[serde(rename = "openid.ns")]
	namespace: &'static str,

	#[serde(rename = "openid.identity")]
	identity: &'static str,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: &'static str,

	#[serde(rename = "openid.mode")]
	mode: &'static str,

	#[serde(rename = "openid.realm")]
	realm: Url,

	#[serde(rename = "openid.return_to")]
	return_to: Url,
}

impl LoginForm {
	/// The route to which a user should be redirected back to by Steam after logging in.
	const RETURN_ROUTE: &'static str = "/auth/callback";

	/// The URL we redirect a user to when they log into Steam.
	///
	/// This is also used for verifying Steam's callback requests.
	const LOGIN_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Creates a new [`LoginForm`] that will redirect back to the given `realm`.
	pub fn new(realm: Url) -> Self {
		let return_to = realm.join(Self::RETURN_ROUTE).expect("this is valid");

		Self {
			namespace: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			realm,
			return_to,
		}
	}

	/// Create a [`Redirect`] to Steam, which will redirect back to `redirect_to` after the
	/// login process is complete.
	pub fn redirect_to(mut self, redirect_to: &Url) -> Redirect {
		self.return_to
			.query_pairs_mut()
			.append_pair("redirect_to", redirect_to.as_str());

		let query_string =
			serde_urlencoded::to_string(&self).expect("this is a valid query string");

		let mut url = Url::parse(Self::LOGIN_URL).expect("this is a valid url");

		url.set_query(Some(&query_string));

		Redirect::to(url.as_str())
	}
}

/// The payload sent by Steam after a user logged in.
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct LoginResponse {
	/// The URL we should redirect the user back to after we verified their request.
	///
	/// This is injected by [`LoginForm::redirect_to()`].
	#[serde(skip_serializing)]
	pub redirect_to: Url,

	#[serde(rename = "openid.ns")]
	namespace: String,

	#[serde(rename = "openid.identity")]
	identity: Option<String>,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: Url,

	#[serde(rename = "openid.mode")]
	mode: String,

	#[serde(rename = "openid.return_to")]
	return_to: Url,

	#[serde(rename = "openid.op_endpoint")]
	op_endpoint: String,

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

impl LoginResponse {
	/// Extracts the user's SteamID.
	pub fn steam_id(&self) -> SteamID {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|segment| segment.parse::<SteamID>().ok())
			.expect("invalid response from steam")
	}

	/// Verifies this payload by making an API request to Steam.
	pub async fn verify(&mut self, http_client: &reqwest::Client) -> Result<SteamID> {
		self.mode = String::from("check_authentication");

		let payload = serde_urlencoded::to_string(&self).map_err(|err| {
			debug!(%err, "invalid steam login payload");
			Error::unauthorized().with_source(err)
		})?;

		let response = http_client
			.post(LoginForm::LOGIN_URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(payload)
			.send()
			.await
			.and_then(Response::error_for_status)?
			.text()
			.await?;

		if response
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_none()
		{
			debug!(%response, "steam login invalid");
			return Err(Error::unauthorized());
		}

		let steam_id = self.steam_id();

		debug!(%steam_id, redirect_to = %self.redirect_to, "user logged in");

		Ok(steam_id)
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for LoginResponse {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let Query(mut login) = Query::<Self>::from_request_parts(parts, &())
			.await
			.map_err(|err| {
				debug!(%err, "missing steam login payload");
				Error::unauthorized().with_source(err)
			})?;

		let steam_id = login.verify(&state.http_client).await.map_err(|err| {
			debug!(%err, "login request did not come from steam");
			Error::unauthorized().with_source(err)
		})?;

		parts.extensions.insert(steam_id);

		Ok(login)
	}
}
