//! Request / Response types for this service.

use std::collections::BTreeMap;

use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use url::Url;

use super::Session;

/// Request payload for logging in with Steam.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LoginRequest
{
	/// URL to redirect to after the login process is complete.
	pub redirect_to: Url,
}

/// Response payload for logging in with Steam.
#[derive(Debug, Deserialize, utoipa::IntoResponses)]
#[response(status = SEE_OTHER, headers(
  ("Location", description = "Steam's OpenID service"),
))]
pub struct LoginResponse
{
	/// OpenID URL to redirect the user to so they can login.
	pub openid_url: Url,
}

impl IntoResponse for LoginResponse
{
	fn into_response(self) -> Response
	{
		Redirect::to(self.openid_url.as_str()).into_response()
	}
}

/// Request payload for logging in with Steam.
#[derive(Debug)]
pub struct LogoutRequest
{
	/// Whether to invalidate all previous sessions, rather than just the
	/// current one.
	pub invalidate_all_sessions: bool,

	/// The user's session.
	pub session: Session,
}

/// Response for `/auth/logout`.
#[derive(Debug)]
pub struct LogoutResponse
{
	/// The cookie jar that contains the reset cookies.
	pub(super) cookies: CookieJar,
}

impl IntoResponse for LogoutResponse
{
	fn into_response(self) -> Response
	{
		self.cookies.into_response()
	}
}

impl utoipa::IntoResponses for LogoutResponse
{
	fn responses() -> BTreeMap<String, utoipa::openapi::RefOr<utoipa::openapi::response::Response>>
	{
		use utoipa::openapi::header::HeaderBuilder;
		use utoipa::openapi::response::{ResponseBuilder, ResponsesBuilder};

		ResponsesBuilder::new()
			.response(
				"200",
				ResponseBuilder::new().header(
					"Set-Cookies",
					HeaderBuilder::new()
						.description(Some("your cleared `kz-*` cookies"))
						.into(),
				),
			)
			.build()
			.into()
	}
}
