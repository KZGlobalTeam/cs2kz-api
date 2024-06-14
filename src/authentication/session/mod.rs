//! Session authentication.
//!
//! This module contains the [`Session`] type, which acts as an [extractor].
//! It implements both [`FromRequestParts`], as well as [`IntoResponseParts`].
//!
//! # Life Cycle
//!
//! The typical life cycle of a session is as follows:
//!    1. A request comes in, with a [session ID] inside a [cookie]
//!    2. [`Session`] acts as an [extractor] via its [`FromRequestParts`] implementation
//!       2.1. The auth [cookie] value will be extracted from the request headers and parsed into a
//!            UUID
//!       2.2. The session is looked up in the database
//!       2.3. The session is authorized by invoking [`AuthorizeSession::authorize_session()`]
//!       2.4. The session is extended both in the database and in the cookie timestamp
//!       2.5. The session is inserted into the request's extensions so we don't run auth logic
//!            more than once
//!    3. The [session ID] and [user] can be accessed by the request handler
//!    4. The request handler runs
//!    5. [`Session`]'s [`IntoResponseParts`] implementation is invoked, and the resulting response
//!       will include a `Set-Cookie` header with the updated information
//!
//! # Invalidating Sessions
//!
//! If a session should be invalidated, like in the [`/auth/logout` handler][logout], you can call
//! [`Session::invalidate()`] before returning it in the response. This will set the session's
//! expiration date to "now" both in the database and the cookie that will be returned to the user.
//!
//! [extractor]: axum::extract
//! [session ID]: SessionID
//! [cookie]: COOKIE_NAME
//! [user]: User
//! [logout]: crate::authentication::handlers::logout

use std::marker::PhantomData;
use std::net::IpAddr;

use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum::{async_trait, http};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use derive_more::{Debug, From, Into};
use sqlx::{MySql, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::authentication::User;
use crate::authorization::{self, AuthorizeSession, Permissions};
use crate::sqlx::SqlErrorExt;
use crate::{steam, Error, Result, State};

mod id;
pub use id::SessionID;

/// The HTTP cookie name that stores the user's [session ID].
///
/// [session ID]: SessionID
pub const COOKIE_NAME: &str = "kz-auth";

/// A user session.
///
/// This type acts as an [extractor] for session authentication.
/// See [module level docs] for more details.
///
/// [extractor]: axum::extract
/// [module level docs]: crate::authentication::session
#[must_use = "sessions are stateful, and creating a new one involves database operations"]
#[derive(Debug, Into)]
pub struct Session<A = authorization::None> {
	/// The session's ID.
	id: SessionID,

	/// The user associated with this session.
	#[debug("{} ({})", user.steam_id(), user.permissions())]
	user: User,

	/// The cookie that was extracted from the user request / will be sent back to the user in
	/// the response.
	#[debug(skip)]
	#[into]
	cookie: Cookie<'static>,

	/// Marker to tie an authorization method to any given [`Session`] without actually storing
	/// anything.
	#[debug(skip)]
	_authorization: PhantomData<A>,
}

impl<A> Session<A> {
	/// Returns this session's ID.
	pub const fn id(&self) -> SessionID {
		self.id
	}

	/// Returns the user associated with this session.
	pub const fn user(&self) -> User {
		self.user
	}

	/// Generates a new expiration date for any given session.
	fn expires_on() -> OffsetDateTime {
		OffsetDateTime::now_utc() + time::Duration::WEEK
	}
}

impl Session {
	/// Creates a new [`Session`].
	///
	/// NOTE: this inserts new data into the database
	#[tracing::instrument(level = "debug", name = "auth::session::login", skip_all, fields(
		user.steam_id = %steam_user.steam_id,
		user.username = %steam_user.username,
		user.ip_address = %user_ip,
		session.id = tracing::field::Empty,
		session.expires_on = tracing::field::Empty,
	))]
	pub async fn create(
		steam_user: &steam::User,
		user_ip: IpAddr,
		api_config: &crate::Config,
		mut transaction: Transaction<'_, MySql>,
	) -> Result<Self> {
		let session_id = SessionID::new();
		let expires_on = Self::expires_on();

		tracing::Span::current()
			.record("session.id", format_args!("{session_id}"))
			.record("session.expires_on", format_args!("{expires_on}"));

		tracing::debug!("generating new session");

		let user_exists = sqlx::query! {
			r#"
			SELECT
			  id
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			steam_user.steam_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.is_some();

		if !user_exists {
			tracing::debug!("user does not exist; inserting default values");

			sqlx::query! {
				r#"
				INSERT INTO
				  Players (id, name, ip_address)
				VALUES
				  (?, ?, ?)
				"#,
				steam_user.steam_id,
				steam_user.username,
				user_ip,
			}
			.execute(transaction.as_mut())
			.await?;
		}

		sqlx::query! {
			r#"
			INSERT INTO
			  LoginSessions (id, player_id, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			session_id,
			steam_user.steam_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				// This should be impossible because of the check above
				Error::logic("player fk violation even though we created the player")
					.context(err)
					.context(format!(
						"steam_id: {}, name: {}",
						steam_user.steam_id, steam_user.username,
					))
			} else {
				Error::from(err)
			}
		})?;

		let user = sqlx::query! {
			r#"
			SELECT
			  permissions `permissions: Permissions`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			steam_user.steam_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| User::new(steam_user.steam_id, row.permissions))
		.ok_or_else(|| Error::unknown("SteamID"))?;

		transaction.commit().await?;

		tracing::debug!("created session");

		let cookie = Cookie::build((COOKIE_NAME, session_id.to_string()))
			.domain(api_config.cookie_domain.clone())
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(true)
			.expires(expires_on)
			.build();

		Ok(Self {
			id: session_id,
			user,
			cookie,
			_authorization: PhantomData,
		})
	}
}

impl<A> Session<A>
where
	A: AuthorizeSession,
{
	/// Invalidate this session.
	///
	/// This will set the session's expiration date to "now", both in the database and the
	/// cookie that will be returned in the response.
	///
	/// If `invalid_all` is `true`, **every** session in the database associated with this
	/// session's user will be invalidated.
	#[tracing::instrument(level = "debug", name = "auth::session::logout", skip(database))]
	pub async fn invalidate(
		&mut self,
		invalidate_all: bool,
		database: &mut Transaction<'_, MySql>,
	) -> Result<()> {
		sqlx::query! {
			r#"
			UPDATE
			  LoginSessions
			SET
			  expires_on = NOW()
			WHERE
			  player_id = ?
			  AND expires_on > NOW()
			  AND (
			    id = ?
			    OR ?
			  )
			"#,
			self.user.steam_id(),
			self.id,
			invalidate_all,
		}
		.execute(database.as_mut())
		.await?;

		self.cookie.set_expires(OffsetDateTime::now_utc());

		let steam_id = self.user.steam_id();
		let message = if invalidate_all {
			"invalidated all sessions for user"
		} else {
			"invalidated session for user"
		};

		tracing::debug!(user.id = %steam_id, "{message}");

		Ok(())
	}
}

#[async_trait]
impl<A> FromRequestParts<State> for Session<A>
where
	A: AuthorizeSession,
{
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::session::from_request_parts",
		skip_all,
		fields(session.id = tracing::field::Empty, session.user.id = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(request: &mut request::Parts, state: &State) -> Result<Self> {
		if let Some(session) = request.extensions.remove::<Self>() {
			tracing::debug!(%session.id, "extracting cached session");
			return Ok(session);
		}

		let (cookie, session_id) = request
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| Cookie::split_parse_encoded(value.trim().to_owned()))
			.flatten()
			.find_map(|cookie| {
				let name = cookie.name();
				let value = cookie.value();

				if name != COOKIE_NAME {
					return None;
				}

				value
					.parse::<Uuid>()
					.inspect_err(|error| {
						tracing::debug! {
							cookie.name = %name,
							cookie.value = %value,
							%error,
							"found cookie but failed to parse value",
						}
					})
					.map(|session_id| (cookie, session_id))
					.ok()
			})
			.ok_or_else(|| Error::missing_session_id())?;

		let current_span = tracing::Span::current();

		current_span.record("session.id", format_args!("{session_id}"));

		let mut transaction = state.transaction().await?;

		tracing::debug!("fetching session from database");

		let session = sqlx::query! {
			r#"
			SELECT
			  s.id `id: SessionID`,
			  p.id `user_id: SteamID`,
			  p.permissions `permissions: Permissions`
			FROM
			  LoginSessions s
			  JOIN Players p ON p.id = s.player_id
			WHERE
			  s.id = ?
			  AND s.expires_on > NOW()
			ORDER BY
			  expires_on DESC
			"#,
			session_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or_else(|| Error::invalid("session ID"))?;

		current_span.record("session.user.id", format_args!("{}", session.user_id));

		tracing::debug!("successfully authenticated session");

		let mut session = Self {
			id: session.id,
			user: User::new(session.user_id, session.permissions),
			cookie,
			_authorization: PhantomData,
		};

		tracing::debug! {
			method = std::any::type_name::<A>().split("::").last().unwrap(),
			"authorizing session",
		};

		A::authorize_session(&session.user, request, &mut transaction).await?;

		let expires_on = Self::expires_on();

		tracing::debug!(until = %expires_on, "extending session");

		session.cookie.set_path("/");
		session.cookie.set_secure(cfg!(feature = "production"));
		session.cookie.set_http_only(true);
		session.cookie.set_expires(expires_on);

		sqlx::query! {
			r#"
			UPDATE
			  LoginSessions
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

		transaction.commit().await?;

		tracing::debug!("extracted session");

		Ok(session)
	}
}

impl<A> IntoResponseParts for Session<A> {
	type Error = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::session::into_response_parts",
		skip_all,
		fields(cookie = tracing::field::Empty),
	)]
	fn into_response_parts(self, mut response: ResponseParts) -> Result<ResponseParts> {
		let cookie = Cookie::from(self)
			.encoded()
			.to_string()
			.parse::<http::HeaderValue>()
			.expect("valid cookie");

		tracing::Span::current().record("cookie", format_args!("{cookie:?}"));
		tracing::debug!("inserting cookie into response headers");

		response.headers_mut().insert(header::SET_COOKIE, cookie);

		Ok(response)
	}
}

impl<A> Clone for Session<A> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			user: self.user,
			cookie: self.cookie.clone(),
			_authorization: PhantomData,
		}
	}
}
