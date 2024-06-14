//! No authorization.
//!
//! This is the default authorization method.

use axum::http::request;
use sqlx::{MySql, Transaction};

use super::AuthorizeSession;
use crate::{authentication, Result};

/// An authorization methods which always succeeds.
#[derive(Debug, Clone, Copy)]
pub struct None;

impl AuthorizeSession for None {
	async fn authorize_session(
		_user: &authentication::User,
		_req: &mut request::Parts,
		_transaction: &mut Transaction<'_, MySql>,
	) -> Result<()> {
		Ok(())
	}
}
