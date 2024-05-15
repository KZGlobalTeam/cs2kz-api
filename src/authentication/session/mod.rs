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

use axum::extract::FromRequestParts;
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum::{async_trait, http};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use derive_more::{Debug, From, Into};
use sqlx::{MySql, Transaction};
use time::OffsetDateTime;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::authentication::User;
use crate::authorization::{self, AuthorizeSession, Permissions};
use crate::sqlx::SqlErrorExt;
use crate::{Error, Result, State};

mod id;

#[doc(inline)]
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
	pub async fn create(
		user_id: SteamID,
		config: &'static crate::Config,
		mut transaction: Transaction<'_, MySql>,
	) -> Result<Self> {
		let session_id = SessionID::new();
		let expires_on = Self::expires_on();

		sqlx::query! {
			r#"
			INSERT INTO
			  LoginSessions (id, player_id, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			session_id,
			user_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::unknown("player").with_source(err)
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
			user_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| User::new(user_id, row.permissions))
		.ok_or_else(|| Error::unknown("SteamID"))?;

		transaction.commit().await?;

		debug!(%session_id, %user_id, "created session");

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

		if invalidate_all {
			debug!(user.id = %steam_id, "invalidated all sessions for user");
		} else {
			debug!(session.id = %self.id, user.id = %steam_id, "invalidated session for user");
		}

		Ok(())
	}
}

#[async_trait]
impl<A> FromRequestParts<&'static State> for Session<A>
where
	A: AuthorizeSession,
{
	type Rejection = Error;

	async fn from_request_parts(
		request: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		if let Some(session) = request.extensions.remove::<Self>() {
			trace!(%session.id, "extracted cached session");
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
					.inspect_err(|err| {
						debug! {
							cookie.name = %name,
							cookie.value = %value,
							%err,
							"found cookie but failed to parse value",
						}
					})
					.map(|session_id| (cookie, session_id))
					.ok()
			})
			.ok_or_else(|| Error::missing_session_id())?;

		let mut transaction = state.transaction().await?;

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

		trace!(%session.id, "fetched session");

		let expires_on = Self::expires_on();

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

		trace!(%session.id, until = %expires_on, "extended session");

		let session = Self {
			id: session.id,
			user: User::new(session.user_id, session.permissions),
			cookie,
			_authorization: PhantomData,
		};

		A::authorize_session(&session.user, request, &mut transaction).await?;

		transaction.commit().await?;

		Ok(session)
	}
}

impl<A> IntoResponseParts for Session<A> {
	type Error = Error;

	fn into_response_parts(self, mut response: ResponseParts) -> Result<ResponseParts> {
		let cookie = Cookie::from(self)
			.encoded()
			.to_string()
			.parse::<http::HeaderValue>()
			.expect("valid cookie");

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
