use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use itertools::Itertools;
use sqlx::MySqlExecutor;
use time::{Duration, OffsetDateTime};
use tracing::trace;

use super::{RoleFlags, User};
use crate::{audit, AppState, Error, Result, State};

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

impl Session {
	/// Generates a new session in the database.
	pub async fn new(steam_id: SteamID, state: AppState) -> Result<Self> {
		let mut transaction = state.begin_transaction().await?;
		let token = rand::random::<u64>();
		let expires_on = OffsetDateTime::now_utc() + Self::EXPIRES_AFTER;

		sqlx::query! {
			r#"
			INSERT INTO
			  WebSessions (token, steam_id, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			token,
			steam_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await?;

		let id = sqlx::query!("SELECT LAST_INSERT_ID() id")
			.fetch_one(transaction.as_mut())
			.await
			.map(|row| row.id)?;

		transaction.commit().await?;

		audit!("session created", %id, %steam_id);

		let user = sqlx::query! {
			r#"
			SELECT
			  role_flags `role_flags: RoleFlags`
			FROM
			  Players
			WHERE
			  steam_id = ?
			"#,
			steam_id,
		}
		.fetch_optional(&state.database)
		.await?
		.map(|row| User::new(steam_id, row.role_flags))
		.ok_or_else(|| Error::unknown("SteamID").with_detail("{steam_id}"))?;

		let cookie = Cookie::build((Self::COOKIE_NAME, token.to_string()))
			.domain(&state.config.domain)
			.path("/")
			.secure(state.config.environment.is_prod())
			.http_only(true)
			.expires(expires_on)
			.build();

		Ok(Self { id, token, user, cookie, invalidated: false })
	}
}

impl<const REQUIRED_FLAGS: u32> Session<REQUIRED_FLAGS> {
	pub const COOKIE_NAME: &'static str = "kz-auth";
	pub const EXPIRES_AFTER: Duration = Duration::WEEK;

	/// Invalidates this session.
	pub async fn invalidate(&mut self, all: bool, executor: impl MySqlExecutor<'_>) -> Result<()> {
		sqlx::query! {
			r#"
			UPDATE
			  WebSessions
			SET
			  expires_on = CURRENT_TIMESTAMP()
			WHERE
			  steam_id = ?
			  AND expires_on > CURRENT_TIMESTAMP()
			  AND (
			    id = ?
			    OR ?
			  )
			"#,
			self.user.steam_id,
			self.id,
			all,
		}
		.execute(executor)
		.await?;

		if all {
			audit!("all sessions invalidated", steam_id = %self.user.steam_id);
		} else {
			audit!("session invalidated", id = %self.id, steam_id = %self.user.steam_id);
		}

		self.invalidated = true;

		Ok(())
	}
}

#[async_trait]
impl<const REQUIRED_FLAGS: u32> FromRequestParts<&'static State> for Session<REQUIRED_FLAGS> {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		if let Some(session) = parts.extensions.remove::<Self>() {
			return Ok(session);
		}

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
			.ok_or_else(|| {
				Error::missing("cookie")
					.with_detail(Self::COOKIE_NAME)
					.unauthorized()
			})?;

		let mut transaction = state.begin_transaction().await?;

		let session = sqlx::query! {
			r#"
			SELECT
			  s.id,
			  p.steam_id `steam_id: SteamID`,
			  p.role_flags `role_flags: RoleFlags`
			FROM
			  WebSessions s
			  JOIN Players p ON p.steam_id = s.steam_id
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
		.ok_or_else(|| Error::missing("valid session").unauthorized())?;

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

		let user = User::new(session.steam_id, session.role_flags);
		let required_flags = RoleFlags::from(REQUIRED_FLAGS);

		if !user.role_flags.contains(required_flags) {
			return Err(Error::missing("required permissions")
				.with_message("you do not have the required permissions to make this request")
				.with_detail(required_flags.into_iter().join(", "))
				.unauthorized());
		}

		audit!("user authenticated", ?user);

		Ok(Self {
			id: session.id,
			token: session_token,
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
