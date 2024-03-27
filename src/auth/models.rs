//! Types related to authentication.

use std::num::NonZeroU16;
use std::result::Result as StdResult;

use axum::async_trait;
use axum::extract::{FromRequestParts, Query};
use axum::http::request;
use axum::response::Redirect;
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use reqwest::{header, Response};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::{error, trace};
use url::Url;
use utoipa::{IntoParams, ToSchema};

use super::RoleFlags;
use crate::{Error, Result, State};

/// An authenticated server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Server {
	/// The server's ID.
	#[schema(value_type = u16)]
	id: NonZeroU16,

	/// The CS2KZ version ID the server is currently running on.
	#[schema(value_type = u16)]
	plugin_version_id: NonZeroU16,
}

impl Server {
	/// Creates a new [`Server`].
	pub const fn new(id: NonZeroU16, plugin_version_id: NonZeroU16) -> Self {
		Self { id, plugin_version_id }
	}

	/// The server's ID.
	pub const fn id(&self) -> NonZeroU16 {
		self.id
	}

	/// The CS2KZ version ID the server is currently running on.
	pub const fn plugin_version_id(&self) -> NonZeroU16 {
		self.plugin_version_id
	}
}

/// An authenticated user.
#[derive(Debug, Clone, Copy)]
pub struct User {
	/// The user's SteamID.
	steam_id: SteamID,

	/// The user's roles as a bitfield.
	role_flags: RoleFlags,
}

impl User {
	/// Creates a new [`User`].
	pub const fn new(steam_id: SteamID, role_flags: RoleFlags) -> Self {
		Self { steam_id, role_flags }
	}

	/// The user's SteamID.
	pub const fn steam_id(&self) -> SteamID {
		self.steam_id
	}

	/// The user's roles as a bitfield.
	pub const fn role_flags(&self) -> RoleFlags {
		self.role_flags
	}
}

/// Form used for logging a user into Steam.
#[derive(Debug, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct SteamLoginForm {
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

impl SteamLoginForm {
	/// The route to which a user should be redirected back to by Steam after logging in.
	const RETURN_ROUTE: &'static str = "/auth/callback";

	/// The URL we redirect a user to when they log into Steam.
	///
	/// This is also used for verifying Steam's callback requests.
	const LOGIN_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Creates a new [`SteamLoginForm`] that will redirect back to the given `realm`.
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
pub struct SteamLoginResponse {
	/// The URL we should redirect the user back to after we verified their request.
	///
	/// This is injected by [`SteamLoginForm::redirect_to()`].
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

impl SteamLoginResponse {
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
			trace!("invalid steam login payload");
			Error::unauthorized().with_source(err)
		})?;

		let response = http_client
			.post(SteamLoginForm::LOGIN_URL)
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
			trace!("steam login invalid");
			return Err(Error::unauthorized());
		}

		let steam_id = self.steam_id();

		trace!(%steam_id, redirect_to = %self.redirect_to, "user logged in");

		Ok(steam_id)
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for SteamLoginResponse {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let Query(mut login) = Query::<Self>::from_request_parts(parts, state)
			.await
			.map_err(|err| {
				trace!(%err, "missing steam login payload");
				Error::unauthorized().with_source(err)
			})?;

		let steam_id = login.verify(&state.http_client).await.map_err(|err| {
			trace!("login request did not come from steam");
			Error::unauthorized().with_source(err)
		})?;

		parts.extensions.insert(steam_id);

		Ok(login)
	}
}

/// Information about a Steam user.
///
/// This will be serialized as JSON and put into a cookie so frontends can use it.
#[derive(Debug, Serialize)]
pub struct SteamUser {
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// Also the user's SteamID, but encoded as a stringified 64-bit integer, because
	/// JavaScript.
	pub steam_id64: String,

	/// The user's username.
	pub username: String,

	/// The user's "real" name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub realname: Option<String>,

	/// The user's country.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,

	/// URL to the user's profile.
	pub profile_url: Url,

	/// URL to the user's avatar.
	pub avatar_url: Url,
}

impl SteamUser {
	/// Steam WebAPI URL for fetching information about players.
	const API_URL: &'static str = "http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

	/// The cookie name used to store the user information.
	const COOKIE_NAME: &'static str = "kz-player";

	/// Creates a [`Cookie`] containing this [`SteamUser`] as a JSON value.
	pub fn to_cookie<'c>(&self, config: &'c crate::Config) -> Cookie<'c> {
		let json = serde_json::to_string(self).expect("this is valid json");

		Cookie::build((Self::COOKIE_NAME, json))
			.domain(&config.domain)
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(false)
			.permanent()
			.build()
	}
}

impl<'de> Deserialize<'de> for SteamUser {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			players: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3 {
			steamid: SteamID,
			personaname: String,
			realname: Option<String>,
			loccountrycode: Option<String>,
			profileurl: Url,
			avatar: Url,
		}

		Helper1::deserialize(deserializer).map(|x| x.response).map(
			|Helper2 { players: [player] }| Self {
				steam_id: player.steamid,
				steam_id64: player.steamid.as_u64().to_string(),
				username: player.personaname,
				realname: player.realname,
				country: player.loccountrycode,
				profile_url: player.profileurl,
				avatar_url: player.avatar,
			},
		)
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for SteamUser {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let steam_id = parts
			.extensions
			.get::<SteamID>()
			.copied()
			.expect("`SteamLoginResponse` extractor should have inserted this");

		let url = Url::parse_with_params(Self::API_URL, [
			("key", state.config.steam_api_key.clone()),
			("steamids", steam_id.as_u64().to_string()),
		])
		.map_err(|err| {
			error!(target: "audit_log", %err, "failed to parse url");
			Error::bug("failed to parse url").with_source(err)
		})?;

		state
			.http_client
			.get(url)
			.send()
			.await?
			.json::<Self>()
			.await
			.map_err(|err| Error::from(err))
	}
}
