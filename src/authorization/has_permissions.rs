//! An authorization method that ensures the user has specific permissions.

use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::authorization::Permissions;
use crate::{authentication, Error};

/// Ensure the user has _at least_ `PERMS`.
#[derive(Debug, Clone, Copy)]
pub struct HasPermissions<const PERMS: u32>;

impl<const PERMS: u32> AuthorizeSession for HasPermissions<PERMS> {
	async fn authorize_session(
		user: &authentication::User,
		_req: &mut request::Parts,
		_transaction: &mut Transaction<'static, MySql>,
	) -> crate::Result<()> {
		let permissions = Permissions::new(PERMS);

		if user.permissions().contains(permissions) {
			Ok(())
		} else {
			Err(Error::insufficient_permissions(permissions))
		}
	}
}
