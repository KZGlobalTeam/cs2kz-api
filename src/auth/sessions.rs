use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use sqlx::{MySql, MySqlExecutor, Transaction};
use time::{Duration, OffsetDateTime};
use tracing::trace;
use url::Url;

use super::{RoleFlags, User};
use crate::url::UrlExt;
use crate::{audit, middleware, Error, Result};

#[derive(Debug, Clone)]
pub struct Session<const REQUIRED_FLAGS: u32 = 0> {
	/// Unique ID for this session.
	pub id: u64,

	/// Randomly generated session token.
	pub token: u64,

	/// The authenticated user.
	pub user: User,

	/// Cookie used for storing the [`token`] in the client's browser.
	///
	/// [`token`]: Self::token
	pub cookie: Cookie<'static>,

	/// Remove [`cookie`] in the [`IntoResponseParts`] impl.
	///
	/// [`cookie`]: Self::cookie
	invalidated: bool,
}

impl<const REQUIRED_FLAGS: u32> Session<REQUIRED_FLAGS> {
	pub const COOKIE_NAME: &'static str = "kz-auth";
	pub const EXPIRES_AFTER: Duration = Duration::WEEK;

	/// Generates a new session in the database.
	pub async fn new(
		steam_id: SteamID,
		url: &Url,
		in_prod: bool,
		transaction: &mut Transaction<'static, MySql>,
	) -> Result<Self> {
		let token = rand::random::<u64>();

		sqlx::query! {
			r#"
			INSERT INTO
			  WebSessions (token, subdomain, steam_id)
			VALUES
			  (?, ?, ?)
			"#,
			token,
			url.subdomain(),
			steam_id,
		}
		.execute(transaction.as_mut())
		.await?;

		let id = sqlx::query!("SELECT LAST_INSERT_ID() id")
			.fetch_one(transaction.as_mut())
			.await
			.map(|row| row.id)?;

		audit!("session created", %id, %steam_id);

		let role_flags = url
			.subdomain()
			.map(RoleFlags::for_subdomain)
			.unwrap_or_default();

		let user = User::new(steam_id, role_flags);

		let domain = url
			.host_str()
			.map(ToOwned::to_owned)
			.expect("API url should have a host");

		let cookie = Cookie::build((Self::COOKIE_NAME, token.to_string()))
			.domain(domain)
			.path("/")
			.secure(in_prod)
			.http_only(true)
			.expires(OffsetDateTime::now_utc() + Self::EXPIRES_AFTER)
			.build();

		let remove_cookie = false;

		Ok(Self { id, token, user, cookie, invalidated: remove_cookie })
	}

	/// Invalidates this session.
	pub async fn invalidate(&mut self, executor: impl MySqlExecutor<'_>) -> Result<()> {
		sqlx::query! {
			r#"
			UPDATE
			  WebSessions
			SET
			  expires_on = CURRENT_TIMESTAMP()
			WHERE
			  id = ?
			"#,
			self.id,
		}
		.execute(executor)
		.await?;

		audit!("session invalidated", id = %self.id, steam_id = %self.user.steam_id);

		self.invalidated = true;

		Ok(())
	}
}

#[async_trait]
impl<const REQUIRED_FLAGS: u32> FromRequestParts<Arc<crate::State>> for Session<REQUIRED_FLAGS> {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &Arc<crate::State>,
	) -> Result<Self> {
		let (mut cookie, session_token) = parts
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| value.split(';'))
			.flat_map(|value| Cookie::parse_encoded(value.trim().to_owned()))
			.find_map(|cookie| {
				if cookie.name() != Self::COOKIE_NAME {
					return None;
				}

				let Ok(session_token) = cookie.value().parse::<u64>() else {
					trace!(?cookie, "cookie has invalid value");
					return None;
				};

				Some((cookie, session_token))
			})
			.ok_or(Error::Unauthorized)?;

		let mut transaction = state.transaction().await?;

		let session = sqlx::query! {
			r#"
			SELECT
			  s.id,
			  s.token,
			  s.subdomain,
			  u.steam_id `steam_id: SteamID`,
			  u.role_flags
			FROM
			  WebSessions s
			  JOIN Players u ON u.steam_id = s.steam_id
			WHERE
			  s.token = ?
			  AND s.expires_on > CURRENT_TIMESTAMP()
			ORDER BY
			  s.expires_on DESC
			"#,
			session_token,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or_else(|| {
			trace!("no valid session found");
			Error::Unauthorized
		})?;

		audit!("session authenticated", id = %session.id);

		let expires_on = OffsetDateTime::now_utc() + Self::EXPIRES_AFTER;

		cookie.set_path("/");
		cookie.set_expires(expires_on);

		sqlx::query! {
			r#"
			UPDATE
			  WebSessions
			SET
			  expires_on = ?
			WHERE
			  id = ?
			"#,
			expires_on,
			session.id,
		}
		.execute(transaction.as_mut())
		.await?;

		audit!("session extended", id = %session.id);

		transaction.commit().await?;

		let mut legal_role_flags = session
			.subdomain
			.as_deref()
			.map(RoleFlags::for_subdomain)
			.unwrap_or_default();

		if state.in_dev() {
			legal_role_flags = RoleFlags::ALL;
		}

		let user = User::new(session.steam_id, session.role_flags & legal_role_flags);
		let required_flags = RoleFlags(REQUIRED_FLAGS);

		if !user.role_flags.contains(required_flags) {
			return Err(middleware::Error::InsufficientPermissions { required_flags }.into());
		}

		audit!("user authenticated", ?user);

		Ok(Self {
			id: session.id,
			token: session.token,
			user,
			cookie,
			invalidated: false,
		})
	}
}

impl<const REQUIRED_FLAGS: u32> IntoResponseParts for Session<REQUIRED_FLAGS> {
	type Error = Error;

	fn into_response_parts(mut self, mut response: ResponseParts) -> Result<ResponseParts> {
		if self.invalidated {
			self.cookie.set_expires(OffsetDateTime::now_utc());
		}

		let cookie = self
			.cookie
			.encoded()
			.to_string()
			.parse()
			.expect("this is a valid cookie");

		response.headers_mut().insert(header::SET_COOKIE, cookie);

		Ok(response)
	}
}
