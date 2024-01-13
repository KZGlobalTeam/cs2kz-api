use axum::response::Redirect;
use serde::Serialize;
use url::Url;

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
	pub fn with_origin_url(mut self, origin_url: Url) -> Redirect {
		self.callback_url
			.query_pairs_mut()
			.append_pair("origin_url", origin_url.as_str());

		let query = serde_urlencoded::to_string(&self).expect("this is a valid query string");
		let mut url = Url::parse(Self::REDIRECT_URL).expect("this is a valid url");

		url.set_query(Some(&query));

		Redirect::to(url.as_str())
	}
}
