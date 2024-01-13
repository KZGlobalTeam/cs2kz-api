use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum_extra::extract::cookie::Cookie;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{Error, Result};

pub type State = axum::extract::State<std::sync::Arc<crate::State>>;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct SessionToken(pub u64);

impl SessionToken {
	pub const COOKIE_NAME: &'static str = "kz-auth";

	/// Generates a new random token.
	pub fn random() -> Self {
		Self(rand::random())
	}
}

impl TryFrom<&Cookie<'_>> for SessionToken {
	type Error = Error;

	fn try_from(cookie: &Cookie<'_>) -> Result<Self> {
		if cookie.name() != Self::COOKIE_NAME || is_expired(cookie) {
			return Err(Error::Unauthorized);
		}

		let token = cookie
			.value()
			.parse::<u64>()
			.map_err(|_| Error::Unauthorized)?;

		Ok(Self(token))
	}
}

#[async_trait]
impl FromRequestParts<Arc<crate::State>> for SessionToken {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &Arc<crate::State>,
	) -> Result<Self> {
		parts
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.filter_map(|value| value.to_str().ok())
			.flat_map(|value| value.split(';'))
			.filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok())
			.find_map(|cookie| Self::try_from(&cookie).ok())
			.ok_or(Error::Unauthorized)
	}
}

fn is_expired(cookie: &Cookie<'_>) -> bool {
	match cookie.expires_datetime() {
		None => false,
		Some(datetime) => datetime < OffsetDateTime::now_utc(),
	}
}
