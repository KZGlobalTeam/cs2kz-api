//! This module holds middleware functions for authentication.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::extractors::SessionToken;
use crate::{audit, Error, Result, State};

mod gameserver;
pub use gameserver::verify_gameserver;

pub mod web;
pub use web::verify_web_user;

/// This will attempt both [`verify_gameserver`] and [`verify_web_user`] before denying a
/// request.
#[tracing::instrument(skip_all, ret, err(Debug))]
pub async fn verify_game_server_or_web_user<const MIN_PERMS: u64>(
	state: State,
	token: Option<TypedHeader<Authorization<Bearer>>>,
	session_token: Option<SessionToken>,
	request: Request,
	next: Next,
) -> Result<Response> {
	match (token, session_token) {
		(None, Some(session_token)) => {
			verify_web_user::<MIN_PERMS>(state, session_token, request, next).await
		}

		(Some(token), None) => gameserver::verify_gameserver(state, token, request, next).await,

		(None, None) => Err(Error::Unauthorized),
		(Some(token), Some(session_token)) => {
			audit!(?token, ?session_token, "got both gameserver jwt and session token");
			Err(Error::Unauthorized)
		}
	}
}
