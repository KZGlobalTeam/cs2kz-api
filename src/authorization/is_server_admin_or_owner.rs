//! Authorization for `/servers` routes, checking if the requesting user is either an admin or the
//! owner of the server that is being modified.

use axum::extract::{FromRequestParts, Path};
use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::authorization::{self, Permissions};
use crate::servers::ServerID;
use crate::{authentication, Error, Result};

/// An authorization method that checks if the requesting user is either an admin with the
/// [`SERVERS`] permission, or the owner of the server that is supposed to be modified by the
/// request.
///
/// [`SERVERS`]: Permissions::SERVERS
#[derive(Debug, Clone, Copy)]
pub struct IsServerAdminOrOwner;

impl AuthorizeSession for IsServerAdminOrOwner {
	#[tracing::instrument(
		level = "debug",
		name = "auth::is_server_admin_or_owner",
		skip_all,
		fields(
			user.id = %user.steam_id(),
			user.permissions = %user.permissions(),
			has_required_permissions = tracing::field::Empty,
			server.id = tracing::field::Empty,
			is_server_owner = tracing::field::Empty,
		),
	)]
	async fn authorize_session(
		user: &authentication::User,
		req: &mut request::Parts,
		transaction: &mut Transaction<'_, MySql>,
	) -> Result<()> {
		let current_span = tracing::Span::current();

		if authorization::HasPermissions::<{ Permissions::SERVERS.value() }>::authorize_session(
			user,
			req,
			transaction,
		)
		.await
		.is_ok()
		{
			current_span.record("has_required_permissions", true);

			return Ok(());
		}

		let Path(server_id) = Path::<ServerID>::from_request_parts(req, &()).await?;

		current_span.record("server.id", format_args!("{server_id}"));

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

		current_span.record("is_server_owner", server_exists);

		if !server_exists {
			return Err(Error::must_be_server_owner());
		}

		Ok(())
	}
}
