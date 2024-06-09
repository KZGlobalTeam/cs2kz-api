//! Ensures the requesting user is either an admin with the `SERVERS` permissions, or the owner of
//! the server they are making a request for.

use axum::extract::{FromRequestParts, Path};
use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::authorization::{self, Permissions};
use crate::servers::ServerID;
use crate::{authentication, Error, Result};

/// Ensures the requesting user is either an admin with the `SERVERS` permissions, or the owner of
/// the server they are making a request for.
#[derive(Debug, Clone, Copy)]
pub struct IsServerAdminOrOwner;

impl AuthorizeSession for IsServerAdminOrOwner {
	async fn authorize_session(
		user: &authentication::User,
		req: &mut request::Parts,
		transaction: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		if authorization::HasPermissions::<{ Permissions::SERVERS.value() }>::authorize_session(
			user,
			req,
			transaction,
		)
		.await
		.is_ok()
		{
			return Ok(());
		}

		let Path(server_id) = Path::<ServerID>::from_request_parts(req, &()).await?;

		let server_exists = sqlx::query! {
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
		.fetch_optional(transaction.as_mut())
		.await?
		.is_some();

		if !server_exists {
			return Err(Error::must_be_server_owner());
		}

		Ok(())
	}
}
