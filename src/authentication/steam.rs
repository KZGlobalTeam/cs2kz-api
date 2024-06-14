//! Steam OpenID authentication.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum::response::Redirect;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::IntoParams;

use crate::{Error, Result, State};

/// Form parameters that will be sent to Steam when redirecting a user for login.
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
	/// The API route that Steam should redirect back to after a successful login.
	const RETURN_ROUTE: &'static str = "/auth/callback";

	/// Steam URL to redirect the user in for login.
	const LOGIN_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Creates a new [`LoginForm`].
	///
	/// `realm` is the base URL of the API.
	#[tracing::instrument(level = "debug", name = "auth::steam::login_form", fields(
		realm = %realm,
		return_to = tracing::field::Empty
	))]
	pub fn new(realm: Url) -> Self {
		let return_to = realm.join(Self::RETURN_ROUTE).expect("this is valid");

		tracing::Span::current().record("return_to", format_args!("{return_to}"));

		Self {
			namespace: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			realm,
			return_to,
		}
	}

	/// Creates a [`Redirect`] that will redirect a request to Steam so the user can login.
	#[tracing::instrument(level = "debug", name = "auth::steam::redirect", skip(self))]
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

/// Form parameters that Steam will send to us after a successful login.
///
/// These can be sent back to Steam for validation, see [`LoginResponse::verify()`].
#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct LoginResponse {
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

impl LoginResponse {
	/// Verifies this payload with Steam and extracts the user's SteamID from it.
	#[tracing::instrument(level = "debug", skip_all, ret, fields(
		redirect_to = %self.redirect_to
	))]
	pub async fn verify(&mut self, http_client: &reqwest::Client) -> Result<SteamID> {
		self.mode = String::from("check_authentication");

		let response = http_client
			.post(LoginForm::LOGIN_URL)
			.form(self)
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
			tracing::debug!(%response, "steam login invalid");
			return Err(Error::unauthorized());
		}

		tracing::debug!("user logged in");

		Ok(self.steam_id())
	}

	/// Extracts the SteamID from this form.
	fn steam_id(&self) -> SteamID {
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|segment| segment.parse::<SteamID>().ok())
			.expect("invalid response from steam")
	}
}

#[async_trait]
impl FromRequestParts<State> for LoginResponse {
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::steam::from_request_parts",
		skip_all,
		fields(steam_id = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(parts: &mut request::Parts, state: &State) -> Result<Self> {
		let Query(mut login) = Query::<Self>::from_request_parts(parts, &())
			.await
			.map_err(|err| {
				Error::unauthorized()
					.context("missing steam login payload")
					.context(err)
			})?;

		let steam_id = login.verify(&state.http_client).await.map_err(|err| {
			Error::unauthorized()
				.context("login request did not come from steam")
				.context(err)
		})?;

		tracing::Span::current().record("steam_id", format_args!("{steam_id}"));

		parts.extensions.insert(steam_id);

		Ok(login)
	}
}
