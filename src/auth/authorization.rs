//! Session authorization.

use std::future::Future;
use std::marker::PhantomData;

use axum::extract::{FromRequestParts, Path};
use axum::http::request;
use sqlx::{MySql, Transaction};

use crate::auth::{RoleFlags, User};
use crate::{Error, Result};

/// Session authorization.
///
/// This trait is used in combination with the [`Session`] type in middleware.
/// It determines how to authorize a given [`Session`].
///
/// [`Session`]: crate::auth::Session
pub trait AuthorizeSession: Send + Sync + 'static {
	/// Authorize a session for the given `user`.
	fn authorize(
		user: &User,
		request: &mut request::Parts,
		database: &mut Transaction<'static, MySql>,
	) -> impl Future<Output = Result<()>> + Send;
}

/// No authorization.
#[derive(Debug)]
pub struct None;

impl AuthorizeSession for None {
	async fn authorize(
		_user: &User,
		_request: &mut request::Parts,
		_database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		Ok(())
	}
}

/// Ensures the user has a given set of roles.
pub struct HasRoles<const ROLE_FLAGS: u32>;

impl<const ROLE_FLAGS: u32> AuthorizeSession for HasRoles<ROLE_FLAGS> {
	async fn authorize(
		user: &User,
		_request: &mut request::Parts,
		_database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		let flags = RoleFlags::from(ROLE_FLAGS);

		match user.role_flags().contains(flags) {
			true => Ok(()),
			false => Err(Error::missing_roles(flags)),
		}
	}
}

/// Ensures the user is a server owner.
///
/// This assumes there is a "server ID" path parameter and will fail if it can't be extracted.
pub struct ServerOwner;

impl AuthorizeSession for ServerOwner {
	async fn authorize(
		user: &User,
		request: &mut request::Parts,
		database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		let Path(server_id) = Path::<u16>::from_request_parts(request, &()).await?;

		let _query_result = sqlx::query! {
			r#"
			SELECT
			  id
			FROM
			  Servers
			WHERE
			  id = ?
			  AND owner_id = ?
			"#,
			server_id,
			user.steam_id(),
		}
		.fetch_optional(database.as_mut())
		.await?
		.ok_or_else(|| Error::must_be_server_owner())?;

		Ok(())
	}
}

/// Helper for trying multiple authorization strategies.
///
/// If `A` fails, `B` will be used instead.
/// If both fail, the request will be rejected.
pub struct Either<A, B> {
	/// Marker so `A` and `B` are used.
	_marker: PhantomData<(A, B)>,
}

impl<A, B> AuthorizeSession for Either<A, B>
where
	A: AuthorizeSession,
	B: AuthorizeSession,
{
	async fn authorize(
		user: &User,
		request: &mut request::Parts,
		database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		if let Ok(()) = A::authorize(user, request, database).await {
			return Ok(());
		}

		B::authorize(user, request, database).await
	}
}
