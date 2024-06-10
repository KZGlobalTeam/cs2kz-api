//! This module contains the [`Session`] type and [`AuthorizeSession`] trait, which are used for
//! managing user sessions on websites such as <https://dashboard.cs2kz.org>.
//!
//! [`Session`] implements both [`FromRequestParts`] as well as [`IntoResponseParts`], which means
//! it acts as an [extractor] and can be returned from handlers. When it is extracted, it fetches
//! various information about the user from the database, and when it is returned from a handler,
//! it extends their session. [`Session::invalidate()`] can be used to invalidate the session
//! instead.
//!
//! Middleware to extend sessions on every request is available via [`middleware::auth::layer()`]
//! and the [`session_auth!()`] helper macro.
//!
//! The basic lifecycle of this type is as follows:
//!    1. The user makes a request with a `kz-auth` header
//!    2. The [`Session`] extractor will parse the value of that header and query the database
//!    3. The session entry in the database and the [`cookie`] will have their expiration dates
//!       extended
//!    4. If desired, one can call [`Session::invalidate()`], which will revert step 3
//!    5. When a [`Session`] is returned from a handler, it will set the `SET_COOKIE` header to
//!       extend the session in the user's browser.
//!
//! The [`AuthorizeSession`] trait is used to determine whether the user should be allowed to
//! proceed with a given request. The default implementation does nothing, but can be overridden by
//! specifying the generic. All types which implement this trait are defined in [`authorization`].
//!
//! [extractor]: axum::extract
//! [`middleware::auth::layer()`]: crate::middleware::auth::layer
//! [`session_auth!()`]: crate::middleware::auth::session_auth
//! [`cookie`]: Session::cookie
//! [`authorization`]: crate::authorization

#![allow(rustdoc::private_intra_doc_links)]

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

/// The name of the cookie holding the session's ID in the user's browser.
pub const COOKIE_NAME: &str = "kz-auth";

/// An authenticated session.
///
/// This type acts as an [extractor] to protect handlers.
/// The [`Session::create()`] function is used to create a new session **in the database**.
/// The resulting [`Session`] instance can then be returned from a middleware / handler to
/// propagate updates to the expiration date to the user.
///
/// [extractor]: axum::extract
#[must_use]
#[derive(Debug, Into)]
pub struct Session<A = authorization::None> {
	/// The session ID.
	id: SessionID,

	/// The user associated with this session.
	#[debug("{} ({})", user.steam_id(), user.permissions())]
	user: User,

	/// The cookie that was extracted / will be sent.
	#[debug(skip)]
	#[into]
	cookie: Cookie<'static>,

	/// Marker for encoding the authorization method in the session's type.
	#[debug(skip)]
	_authorization: PhantomData<A>,
}

impl<A> Session<A> {
	/// Returns the session ID.
	pub const fn id(&self) -> SessionID {
		self.id
	}

	/// Returns the logged-in user.
	pub const fn user(&self) -> User {
		self.user
	}

	/// Returns the expiration date for a new [Session].
	fn expires_on() -> OffsetDateTime {
		OffsetDateTime::now_utc() + time::Duration::WEEK
	}
}

impl Session {
	/// Create a new session.
	///
	/// This will create a new session for the given user in the database.
	/// The returned value of this function should be returned from a handler / middleware so a
	/// cookie can be sent to the user's browser.
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
		config: &'static crate::Config,
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
			.domain(config.cookie_domain.clone())
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
	/// Invalidates the given session.
	///
	/// This invovles both updating the database record as well as the stored cookie.
	/// To propagate the change to the user, `self` must be returned from a handler /
	/// middleware.
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
impl<A> FromRequestParts<&'static State> for Session<A>
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
	async fn from_request_parts(
		request: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		if let Some(session) = request.extensions.remove::<Self>() {
			tracing::debug!(%session.id, "extracting cached session");
			return Ok(session);
		}

		let (mut cookie, session_id) = request
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
						tracing::warn! {
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
		.ok_or_else(|| Error::invalid_session_id())?;

		current_span.record("session.user.id", format_args!("{}", session.user_id));

		let expires_on = Self::expires_on();

		tracing::debug!(until = %expires_on, "extending session");

		cookie.set_path("/");
		cookie.set_secure(cfg!(feature = "production"));
		cookie.set_http_only(true);
		cookie.set_expires(expires_on);

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

		tracing::debug!("successfully authenticated session");

		let session = Self {
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
