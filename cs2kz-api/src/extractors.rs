//! Custom [extractors].
//!
//! [extractors]: axum::extract

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum_extra::extract::cookie::Cookie;
use time::OffsetDateTime;

use crate::{AppState, Error, Result};

/// A session token for logged in users.
#[derive(Debug)]
pub struct SessionToken(pub u64);

#[async_trait]
impl FromRequestParts<&'static AppState> for SessionToken {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut request::Parts, _: &&'static AppState) -> Result<Self> {
		parts
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.filter_map(|value| value.to_str().ok())
			.flat_map(|value| value.split(';'))
			.filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok())
			.find(|cookie| cookie.name() == "kz-auth" && !is_expired(cookie))
			.and_then(|cookie| cookie.value().parse::<u64>().ok())
			.map(Self)
			.ok_or(Error::Unauthorized)
	}
}

fn is_expired(cookie: &Cookie<'_>) -> bool {
	match cookie.expires_datetime() {
		None => false,
		Some(datetime) => datetime < OffsetDateTime::now_utc(),
	}
}
