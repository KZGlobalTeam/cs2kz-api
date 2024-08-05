//! Session authentication.
//!
//! For a quick overview, see the [`auth` top-level documentation].
//!
//! [`auth` top-level documentation]: crate::services::auth

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum_extra::extract::cookie::{Cookie, SameSite};
use cs2kz::SteamID;
use http::{header, request};
use sqlx::{MySql, Pool};

mod id;
pub use id::SessionID;

pub mod user;
pub use user::User;

mod rejection;
pub use rejection::SessionRejection;

pub mod authorization;
pub use authorization::AuthorizeSession;

mod service;
pub use service::{SessionManager, SessionManagerLayer};

/// The name of the HTTP cookie that will store the user's [session ID].
///
/// [session ID]: SessionID
pub const COOKIE_NAME: &str = "kz-auth";

/// An authenticated session.
///
/// This struct represents a session that has either just been created, or
/// extracted from a request.
#[derive(Debug, Clone)]
pub struct Session
{
	/// The session's ID.
	id: SessionID,

	/// The user associated with this session.
	user: User,
}

impl Session
{
	/// Creates a new [`Session`].
	#[tracing::instrument(level = "trace")]
	pub(super) fn new(id: SessionID, user: User) -> Self
	{
		Self { id, user }
	}

	/// Returns this session's ID.
	pub fn id(&self) -> SessionID
	{
		self.id
	}

	/// Returns the user associated with this session.
	pub fn user(&self) -> &User
	{
		&self.user
	}

	/// Creates an HTTP cookie from this session.
	pub fn into_cookie(self, domain: impl Into<String>) -> Cookie<'static>
	{
		Cookie::build((COOKIE_NAME, self.id().to_string()))
			.domain(domain.into())
			.path("/")
			.secure(cfg!(feature = "production"))
			.same_site(SameSite::Lax)
			.http_only(true)
			.expires(super::generate_session_expiration_date())
			.build()
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync + 'static,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = SessionRejection;

	#[tracing::instrument(
		name = "Session::from_request_parts",
		skip_all,
		fields(session.id = tracing::field::Empty),
		err(Debug, level = "debug")
	)]
	async fn from_request_parts(
		req: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		if let Some(session) = req.extensions.remove::<Self>() {
			tracing::Span::current().record("session.id", format_args!("{}", session.id()));

			return Ok(session);
		}

		let session_id = req
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| Cookie::split_parse_encoded(value.trim().to_owned()))
			.flatten()
			.find(|cookie| cookie.name() == COOKIE_NAME)
			.map(|cookie| cookie.value().parse::<SessionID>())
			.ok_or(SessionRejection::MissingCookie)?
			.map_err(|source| SessionRejection::ParseSessionID { source })?;

		tracing::Span::current().record("session.id", format_args!("{session_id}"));

		let database = Pool::<MySql>::from_ref(state);
		let session = sqlx::query! {
			r"
			SELECT
			  u.id `user_id: SteamID`,
			  u.permissions `user_permissions: user::Permissions`
			FROM
			  LoginSessions s
			  JOIN Players u ON u.id = s.player_id
			WHERE
			  s.id = ?
			  AND s.expires_on > NOW()
			ORDER BY
			  expires_on DESC
			",
			session_id,
		}
		.fetch_optional(&database)
		.await?
		.map(|row| Session::new(session_id, User::new(row.user_id, row.user_permissions)))
		.ok_or(SessionRejection::InvalidSessionID)?;

		tracing::trace! {
			session.id = %session.id(),
			user.id = %session.user().steam_id(),
			"authenticated session",
		};

		Ok(session)
	}
}

#[cfg(test)]
mod tests
{
	use axum::extract::Request;
	use axum::RequestExt;
	use cs2kz::SteamID;
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::testing;

	const ALPHAKEKS_ID: SteamID = match SteamID::new(76561198282622073_u64) {
		Some(id) => id,
		None => unreachable!(),
	};

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../../database/fixtures/session.sql")
	)]
	async fn accept_valid_session_id(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let mut req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", format!("kz-auth={}", SessionID::TESTING.to_string()))
			.body(Default::default())?;

		let extracted: Session = req.extract_parts_with_state(&database).await?;

		testing::assert_eq!(extracted.id(), SessionID::TESTING);
		testing::assert_eq!(extracted.user().steam_id(), ALPHAKEKS_ID);

		Ok(())
	}

	#[sqlx::test]
	async fn reject_missing_cookie(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let mut req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Session, _>(&database)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, SessionRejection::MissingCookie);

		Ok(())
	}

	#[sqlx::test]
	async fn reject_malformed_session_id(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let mut req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", "kz-auth=foobarbaz")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Session, _>(&database)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, SessionRejection::ParseSessionID { .. });

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn reject_invalid_session_id(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let mut req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", "kz-auth=00000000-0000-0000-0000-000000000000")
			.body(Default::default())?;

		let extracted = req
			.extract_parts_with_state::<Session, _>(&database)
			.await
			.unwrap_err();

		testing::assert_matches!(extracted, SessionRejection::InvalidSessionID);

		Ok(())
	}
}
