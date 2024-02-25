use std::future::Future;
use std::marker::PhantomData;
use std::num::NonZeroU64;

use axum::async_trait;
use axum::extract::{FromRequestParts, Path};
use axum::http::{header, request};
use axum::response::{IntoResponseParts, ResponseParts};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use itertools::Itertools;
use sqlx::{MySqlExecutor, MySqlPool};
use time::{Duration, OffsetDateTime};
use tracing::trace;

use super::{RoleFlags, User};
use crate::{audit, query, Error, Result, State};

/// A user session.
///
/// One of these is created for every authenticated request, as well as when a user logs in.
#[derive(Debug, Clone)]
pub struct Session<A = ()> {
	/// Unique ID for this session.
	id: u64,

	/// The authenticated user.
	user: User,

	/// Cookie used for storing the session token in the client's browser.
	cookie: Cookie<'static>,

	/// Used for [`Authenticated`] impls
	_marker: PhantomData<A>,
}

/// A trait for verifying [`Session`]s.
pub trait Authenticated: Sized + Send + Sync + 'static {
	/// Verifies whether a given `user`'s session is valid.
	fn verify(
		user: &User,
		database: &MySqlPool,
		request: &mut request::Parts,
	) -> impl Future<Output = Result<()>> + Send;
}

impl Session {
	/// Generates a new session in the database.
	pub async fn new(
		steam_id: SteamID,
		database: &MySqlPool,
		config: &'static crate::Config,
	) -> Result<Self> {
		let mut transaction = database.begin().await?;
		let token = rand::random::<NonZeroU64>();
		let expires_on = OffsetDateTime::now_utc() + Self::EXPIRES_AFTER;

		sqlx::query! {
			r#"
			INSERT INTO
			  WebSessions (token, steam_id, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			token.get(),
			steam_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await?;

		let id = query::last_insert_id::<u64>(transaction.as_mut()).await?;

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
		.fetch_optional(database)
		.await?
		.map(|row| User::new(steam_id, row.role_flags))
		.ok_or_else(|| Error::unknown("SteamID").with_detail("{steam_id}"))?;

		let cookie = Cookie::build((Self::COOKIE_NAME, token.to_string()))
			.domain(&config.domain)
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(true)
			.expires(expires_on)
			.build();

		Ok(Self { id, user, cookie, _marker: PhantomData })
	}
}

impl<A> Session<A> {
	/// The name of the cookie used for storing the session token.
	pub const COOKIE_NAME: &'static str = "kz-auth";

	/// The duration after which any given session will expire.
	pub const EXPIRES_AFTER: Duration = Duration::WEEK;

	/// The unique ID associated with this session.
	pub const fn id(&self) -> u64 {
		self.id
	}

	/// The user associated with this session.
	pub const fn user(&self) -> &User {
		&self.user
	}

	/// Invalidates this session.
	///
	/// This both invalidates it in the database, as well as the [session's cookie], so that it
	/// can be returned from middleware/handlers to also reflect in the user's browser.
	///
	/// [session's cookie]: Session::cookie
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

		self.cookie.set_expires(OffsetDateTime::now_utc());

		Ok(())
	}
}

impl<A> From<Session<A>> for Cookie<'static> {
	fn from(session: Session<A>) -> Self {
		session.cookie
	}
}

#[async_trait]
impl<A> FromRequestParts<&'static State> for Session<A>
where
	A: Authenticated,
{
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		// If a session has been extraced previously (specifically in middleware), we don't
		// want to do it again. The middleware will cache the extracted session in the
		// request's extensions, so we can just take that.
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

				let Ok(session_token) = cookie.value().parse::<NonZeroU64>() else {
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
			session_token.get(),
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
		let session = Self { id: session.id, user, cookie, _marker: PhantomData };

		A::verify(&user, &state.database, parts).await?;

		audit!("user authenticated", ?user);

		Ok(session)
	}
}

impl<A> IntoResponseParts for Session<A> {
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

impl Authenticated for () {
	async fn verify(
		_user: &User,
		_database: &MySqlPool,
		_request: &mut request::Parts,
	) -> Result<()> {
		Ok(())
	}
}

/// Helper for checking 2 conditions on a [`Session`] and allowing either one.
///
/// `A` will always be checked first.
#[derive(Debug)]
pub struct Either<A, B> {
	_marker: PhantomData<(A, B)>,
}

impl<A, B> Authenticated for Either<A, B>
where
	A: Authenticated,
	B: Authenticated,
{
	async fn verify(user: &User, database: &MySqlPool, request: &mut request::Parts) -> Result<()> {
		if let Ok(()) = A::verify(user, database, request).await {
			Ok(())
		} else {
			B::verify(user, database, request).await
		}
	}
}

/// Checks whether the user has certain permissions.
#[derive(Debug, Clone, Copy)]
pub struct Admin<const REQUIRED_FLAGS: u32>;

impl<const REQUIRED_FLAGS: u32> Authenticated for Admin<REQUIRED_FLAGS> {
	async fn verify(
		user: &User,
		_database: &MySqlPool,
		_request: &mut request::Parts,
	) -> Result<()> {
		let required_flags = RoleFlags::from(REQUIRED_FLAGS);

		if !user.role_flags.contains(required_flags) {
			return Err(Error::missing("required permissions")
				.with_message("you do not have the required permissions to make this request")
				.with_detail(required_flags.into_iter().join(", "))
				.unauthorized());
		}

		Ok(())
	}
}

/// Extracts a server ID from the request URI and checks whether the user owns the server with that
/// ID.
#[derive(Debug, Clone, Copy)]
pub struct ServerOwner;

impl Authenticated for ServerOwner {
	async fn verify(user: &User, database: &MySqlPool, request: &mut request::Parts) -> Result<()> {
		let server_id = Path::<u16>::from_request_parts(request, &())
			.await
			.map(|path| path.0)
			.map_err(|rejection| Error::missing("server ID").with_detail(format!("{rejection}")))?;

		let result = sqlx::query! {
			r#"
			SELECT
			  id
			FROM
			  Servers
			WHERE
			  id = ?
			  AND owned_by = ?
			"#,
			server_id,
			user.steam_id,
		}
		.fetch_optional(database)
		.await?;

		if result.is_none() {
			return Err(Error::not_a_server_owner());
		}

		Ok(())
	}
}
