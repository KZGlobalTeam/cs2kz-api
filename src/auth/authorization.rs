//! Session authorization.

use std::future::Future;

use axum::extract::{FromRequestParts, Path};
use axum::http::request;
use sqlx::{MySql, Transaction};

use crate::auth::{RoleFlags, User};
use crate::servers::ServerID;
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
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
pub struct HasRoles<const ROLE_FLAGS: u32>;

impl<const ROLE_FLAGS: u32> AuthorizeSession for HasRoles<ROLE_FLAGS> {
	async fn authorize(
		user: &User,
		_request: &mut request::Parts,
		_database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		let flags = RoleFlags::new(ROLE_FLAGS);

		match user.role_flags().contains(flags) {
			true => Ok(()),
			false => Err(Error::missing_roles(flags)),
		}
	}
}

/// Checks if the requesting user is either an admin with the [`SERVERS`] role, or a server owner.
///
/// [`SERVERS`]: RoleFlags::SERVERS
#[derive(Debug, Clone, Copy)]
pub struct AdminOrServerOwner;

impl AuthorizeSession for AdminOrServerOwner {
	async fn authorize(
		user: &User,
		request: &mut request::Parts,
		database: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		if HasRoles::<{ RoleFlags::SERVERS.value() }>::authorize(user, request, database)
			.await
			.is_ok()
		{
			return Ok(());
		}

		let Path(server_id) = Path::<ServerID>::from_request_parts(request, &()).await?;

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
