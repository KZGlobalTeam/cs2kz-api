//! Authorization bound by [Permissions].

use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::authorization::Permissions;
use crate::{authentication, Error, Result};

/// An authorization method that will enforce the user to have at least `PERMS`.
// NOTE: ideally this const generic would be `Permissions` instead of `u32`, but as of writing
//       this, Rust does not allow it
#[derive(Debug, Clone, Copy)]
pub struct HasPermissions<const PERMS: u32>;

impl<const PERMS: u32> AuthorizeSession for HasPermissions<PERMS> {
	#[tracing::instrument(level = "debug", name = "auth::has_permissions", skip_all, fields(
		user.id = %user.steam_id(),
		user.permissions = %user.permissions(),
		required_permissions = tracing::field::Empty,
	))]
	async fn authorize_session(
		user: &authentication::User,
		_req: &mut request::Parts,
		_transaction: &mut Transaction<'_, MySql>,
	) -> Result<()> {
		let required_permissions = Permissions::new(PERMS);

		tracing::Span::current().record(
			"required_permissions",
			format_args!("{required_permissions}"),
		);

		if user.permissions().contains(required_permissions) {
			Ok(())
		} else {
			Err(Error::insufficient_permissions(required_permissions))
		}
	}
}
