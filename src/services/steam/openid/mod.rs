//! OpenID authentication.
//!
//! Steam can act as an OpenID 2.0 authentication provider.
//! This is a fairly old standard, which is why there are barely any libraries
//! for it. The procedure is pretty simple however, and so we implemented it
//! ourselves:
//!
//! 1. create a [`LoginForm`] containing the information that Steam requires to
//!    authenticate a user and send them back to us
//! 2. redirect the user to Steam with the [`LoginForm`] encoded as query
//!    parameters
//! 3. the user will login as usual
//! 4. the user will be redirected back to the endpoint we originally specified
//!    when creating the [`LoginForm`]
//! 5. we receive a request with an [`OpenIDPayload`] encoded in the query
//!    parameters
//! 6. we send the payload back to Steam to verify that it actually originated
//!    from Steam
//! 7. we extract the user's SteamID from the payload, and do with it whatever
//!    we need to do
//!
//! These types implement the relevant traits to act as [extractors], and most
//! of the logic of how they are used, can be found in the
//! [authentication service].
//!
//! [extractors]: axum::extract
//! [authentication service]: crate::services::auth

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum_extra::extract::Query;
use cs2kz::SteamID;
use http::request;
use serde::{Deserialize, Serialize};
use tap::Tap;
use url::Url;

mod rejection;
pub use rejection::OpenIDRejection;

/// Form parameters that will be sent to Steam when redirecting a user for
/// login.
#[derive(Debug, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct LoginForm
{
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

impl LoginForm
{
	/// The API route that Steam should redirect back to after a successful
	/// login.
	pub const RETURN_ROUTE: &'static str = "/auth/callback";

	/// Steam URL to redirect the user in for login.
	pub const LOGIN_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Creates a new [`LoginForm`].
	///
	/// `realm` is the base URL of the API.
	#[tracing::instrument(level = "trace", name = "LoginForm::new")]
	pub(super) fn new(realm: Url) -> Self
	{
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

	/// Generates an OpenID URL that can be used for logging in with Steam.
	#[tracing::instrument(level = "trace", name = "LoginForm::redirect_to", skip(self))]
	pub fn redirect_to(mut self, redirect_to: &Url) -> Url
	{
		self.return_to
			.query_pairs_mut()
			.append_pair("redirect_to", redirect_to.as_str());

		let query_string =
			serde_urlencoded::to_string(&self).expect("this is a valid query string");

		Url::parse(Self::LOGIN_URL)
			.expect("this is a valid url")
			.tap_mut(|url| url.set_query(Some(&query_string)))
	}
}

/// Form parameters that Steam will send to us after a successful login.
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct OpenIDPayload
{
	/// The injected query parameter that was passed as an argument to
	/// [`LoginForm::redirect_to()`].
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

impl OpenIDPayload
{
	/// Verifies this payload with Steam and extracts the user's SteamID from
	/// it.
	#[tracing::instrument(
		level = "debug",
		name = "OpenIDPayload::verify",
		err(Debug, level = "debug"),
		skip_all,
		fields(redirect_to = %self.redirect_to),
	)]
	async fn verify(mut self, http_client: &reqwest::Client) -> Result<Self, OpenIDRejection>
	{
		self.mode = String::from("check_authentication");

		let response = http_client
			.post(LoginForm::LOGIN_URL)
			.form(&self)
			.send()
			.await
			.and_then(reqwest::Response::error_for_status)?
			.text()
			.await?;

		if response
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_none()
		{
			tracing::debug!(%response, "steam login invalid");
			Err(OpenIDRejection::VerifyOpenIDPayload)?;
		}

		tracing::debug!("user logged in");

		Ok(self)
	}

	/// Extracts the SteamID from this form.
	pub fn steam_id(&self) -> SteamID
	{
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|segment| segment.parse::<SteamID>().ok())
			.expect("invalid response from steam")
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for OpenIDPayload
where
	S: Send + Sync + 'static,
	reqwest::Client: FromRef<S>,
{
	type Rejection = OpenIDRejection;

	#[tracing::instrument(
		level = "trace",
		name = "OpenIDPayload::from_request_parts",
		err(Debug, level = "debug")
		skip_all,
	)]
	async fn from_request_parts(
		req: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		let http_client = reqwest::Client::from_ref(state);
		let payload = Query::<Self>::from_request_parts(req, state)
			.await?
			.0
			.verify(&http_client)
			.await?;

		Ok(payload)
	}
}
