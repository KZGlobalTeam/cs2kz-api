//! No authorization.

use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::{authentication, Result};

/// No authorization.
#[derive(Debug, Clone, Copy)]
pub struct None;

impl AuthorizeSession for None {
	async fn authorize_session(
		_user: &authentication::User,
		_req: &mut request::Parts,
		_transaction: &mut Transaction<'static, MySql>,
	) -> Result<()> {
		Ok(())
	}
}
