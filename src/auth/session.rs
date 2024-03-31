//! This module contains the [`Session`] type and [`AuthorizeSession`] trait, which are used for
//! managing user sessions on websites such as <https://dashboard.cs2.kz>.
//!
//! [`Session`] implements both [`FromRequestParts`] as well as [`IntoResponseParts`], which means
//! it acts as an extractor and can be returned from handlers. When it is extracted, it fetches
//! various information about the user from the database, and when it is returned from a handler,
//! it extends their session. [`Session::invalidate()`] can be used to invalidate the session
//! instead.
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
//! specifying the generic. All types which implement this trait are defined in this module.
//!
//! [`cookie`]: Session::cookie

use std::future::Future;
use std::marker::PhantomData;
use std::num::{NonZeroU16, NonZeroU64};

use axum::async_trait;
use axum::extract::{FromRequestParts, Path};
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use derive_more::{Debug, Into};
use sqlx::{MySqlConnection, MySqlExecutor, MySqlPool};
use time::{Duration, OffsetDateTime};
use tracing::trace;
use uuid::Uuid;

use super::{RoleFlags, User};
use crate::sqlx::SqlErrorExt;
use crate::{Error, Result, State};

/// The primary abstraction over login sessions.
///
/// See [module level documentation] for more details.
///
/// [module level documentation]: crate::auth::session
#[must_use]
#[derive(Debug, Into)]
pub struct Session<Auth = ()> {
	/// The session ID.
	#[debug("{id}")]
	id: NonZeroU64,

	/// The user who logged in.
	#[debug("{}", user.steam_id())]
	user: User,

	/// The cookie that was extracted / will be sent back to the user.
	#[debug(skip)]
	#[into]
	cookie: Cookie<'static>,

	/// Marker so we can be generic over something that implements [`AuthorizeSession`].
	#[debug(skip)]
	_auth: PhantomData<Auth>,
}

/// Used for authorizing sessions.
///
/// After a user logged in, this trait will be used to determine whether they can proceed with
/// their request or not.
pub(super) trait AuthorizeSession: Send + Sync + 'static {
	/// Authorizes the given `user`.
	fn authorize(
		user: &User,
		connection: &mut MySqlConnection,
		request: &mut request::Parts,
	) -> impl Future<Output = Result<()>> + Send;
}

impl<Auth> Session<Auth> {
	/// The cookie name used to store the session token.
	pub const COOKIE_NAME: &'static str = "kz-auth";

	/// Timespan after which a session will expire.
	const EXPIRES_AFTER: Duration = Duration::WEEK;

	/// This session's ID.
	pub const fn id(&self) -> NonZeroU64 {
		self.id
	}

	/// The user associated with this session.
	pub const fn user(&self) -> User {
		self.user
	}
}

impl Session {
	/// Creates a new session in the database for the user with the given `steam_id`.
	///
	/// The returned [`Session`] struct should be returned from a handler / middleware to give
	/// the user their session token via a cookie.
	pub async fn create(
		steam_id: SteamID,
		database: &MySqlPool,
		config: &'static crate::Config,
	) -> Result<Self> {
		let token = Uuid::new_v4();
		let expires_on = OffsetDateTime::now_utc() + Self::EXPIRES_AFTER;
		let mut transaction = database.begin().await?;
		let id = sqlx::query! {
			r#"
			INSERT INTO
			  LoginSessions (player_id, token, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			steam_id,
			token,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await
		.map(crate::sqlx::last_insert_id::<NonZeroU64>)
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::unknown("player").with_source(err)
			} else {
				Error::from(err)
			}
		})??;

		trace!(user.id = %steam_id, session.id = %session_id, "created session");

		let user = sqlx::query! {
			r#"
			SELECT
			  role_flags `role_flags: RoleFlags`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			steam_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| User::new(steam_id, row.role_flags))
		.ok_or_else(|| Error::unknown("SteamID"))?;

		transaction.commit().await?;

		let cookie = Cookie::build((Self::COOKIE_NAME, token.to_string()))
			.domain(&config.domain)
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(true)
			.expires(expires_on)
			.build();

		Ok(Self { id, user, cookie, _auth: PhantomData })
	}

	/// Invalidates this session.
	///
	/// This involves updating the expiration date in the database, as well as in the cookie
	/// that will be returned if this session is used as a return type in a handler /
	/// middleware.
	pub async fn invalidate(&mut self, all: bool, database: impl MySqlExecutor<'_>) -> Result<()> {
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
			self.id.get(),
			all,
		}
		.execute(database)
		.await?;

		if all {
			trace!(user.id = %self.user.steam_id(), "invalidated all sessions for user");
		} else {
			trace!(session.id = %self.id, user.id = %self.user.steam_id(), "invalidated session for user");
		}

		self.cookie.set_expires(OffsetDateTime::now_utc());

		Ok(())
	}
}

#[async_trait]
impl<Auth> FromRequestParts<&'static State> for Session<Auth>
where
	Auth: AuthorizeSession,
{
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		if let Some(session) = parts.extensions.remove::<Self>() {
			return Ok(session);
		}

		let find_cookie = |cookie: Cookie<'static>| {
			if cookie.name() != Self::COOKIE_NAME {
				return None;
			}

			let Ok(token) = cookie.value().parse::<Uuid>() else {
				return None;
			};

			Some((cookie, token))
		};

		let (mut cookie, token) = parts
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| value.split(';'))
			.flat_map(|value| Cookie::parse_encoded(value.trim().to_owned()))
			.find_map(find_cookie)
			.ok_or_else(|| Error::missing_session_token())?;

		let mut transaction = state.database.begin().await?;

		let session = sqlx::query! {
			r#"
			SELECT
			  s.id,
			  p.id `user_id: SteamID`,
			  p.role_flags `role_flags: RoleFlags`
			FROM
			  LoginSessions s
			  JOIN Players p ON p.id = s.player_id
			WHERE
			  s.token = ?
			  AND s.expires_on > NOW()
			ORDER BY
			  expires_on DESC
			"#,
			token,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or_else(|| Error::invalid_session_token())?;

		trace!(%session.id, "authenticated session");

		let expires_on = OffsetDateTime::now_utc() + Self::EXPIRES_AFTER;

		cookie.set_path("/");
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

		trace!(%session.id, "extended session");

		let user = User::new(session.user_id, session.role_flags);

		Auth::authorize(&user, transaction.as_mut(), parts).await?;

		let id = NonZeroU64::new(session.id).expect("PKs are never 0");

		transaction.commit().await?;

		Ok(Self { id, user, cookie, _auth: PhantomData })
	}
}

impl<Auth> IntoResponseParts for Session<Auth> {
	type Error = Error;

	fn into_response_parts(self, mut response: ResponseParts) -> Result<ResponseParts> {
		let cookie = Cookie::from(self)
			.encoded()
			.to_string()
			.parse()
			.expect("this is a valid cookie");

		response.headers_mut().insert(header::SET_COOKIE, cookie);

		Ok(response)
	}
}

impl AuthorizeSession for () {
	async fn authorize(
		_user: &User,
		_connection: &mut MySqlConnection,
		_request: &mut request::Parts,
	) -> Result<()> {
		Ok(())
	}
}

/// Authorization method that checks whether the user has a given set of [`RoleFlags`].
pub struct HasRoles<const FLAGS: u32>;

impl<const FLAGS: u32> AuthorizeSession for HasRoles<FLAGS> {
	async fn authorize(
		user: &User,
		_connection: &mut MySqlConnection,
		_request: &mut request::Parts,
	) -> Result<()> {
		let flags = RoleFlags::from(FLAGS);

		if !user.role_flags().contains(flags) {
			return Err(Error::missing_roles(flags));
		}

		Ok(())
	}
}

/// Authorization method that checks whether the user is the owner of a server.
///
/// This is done by extracting a path parameter and doing a database lookup, which means it will
/// probably fail if used incorrectly.
pub struct IsServerOwner;

impl AuthorizeSession for IsServerOwner {
	async fn authorize(
		user: &User,
		connection: &mut MySqlConnection,
		request: &mut request::Parts,
	) -> Result<()> {
		let Path(server_id) = Path::<NonZeroU16>::from_request_parts(request, &()).await?;

		let query_result = sqlx::query! {
			r#"
			SELECT
			  id
			FROM
			  Servers
			WHERE
			  id = ?
			  AND owner_id = ?
			"#,
			server_id.get(),
			user.steam_id(),
		}
		.fetch_optional(connection)
		.await?;

		if query_result.is_none() {
			return Err(Error::must_be_server_owner());
		}

		Ok(())
	}
}

/// Helper for applying more than one authorization method via [`AuthorizeSession`].
///
/// Both types are used in order and the first one to return `true` will authorize the session.
/// If both fail, the request is rejected.
pub struct Either<A, B>(PhantomData<(A, B)>);

impl<A, B> AuthorizeSession for Either<A, B>
where
	A: AuthorizeSession,
	B: AuthorizeSession,
{
	async fn authorize(
		user: &User,
		connection: &mut MySqlConnection,
		request: &mut request::Parts,
	) -> Result<()> {
		if A::authorize(user, connection, request).await.is_ok() {
			Ok(())
		} else {
			B::authorize(user, connection, request).await
		}
	}
}
