use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use jsonwebtoken as jwt;

use super::jwt::GameServerToken;
use crate::{Error, Result, State};

#[tracing::instrument(skip(state, request, next))]
pub async fn auth_server(
	state: State,
	api_token: TypedHeader<Authorization<Bearer>>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let token = state
		.jwt()
		.decode::<GameServerToken>(api_token.token())?
		.claims;

	if token.expires_at < jwt::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	request.extensions_mut().insert(token);

	Ok(next.run(request).await)
}
