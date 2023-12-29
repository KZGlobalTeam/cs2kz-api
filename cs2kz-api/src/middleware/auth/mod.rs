//! This module holds middleware functions for authentication.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::{Result, State};

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
	cookies: CookieJar,
	request: Request,
	next: Next,
) -> Result<Response> {
	// Only gameserver requests will have a bearer token, so we decide which one to call based
	// on that.
	match token {
		None => verify_web_user::<MIN_PERMS>(state, cookies, request, next).await,
		Some(token) => gameserver::verify_gameserver(state, token, request, next).await,
	}
}
