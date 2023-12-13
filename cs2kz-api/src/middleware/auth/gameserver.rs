use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use crate::jwt::ServerClaims;
use crate::{Error, Result, State};

/// Verifies a request coming from a CS2KZ server.
#[tracing::instrument(skip_all, fields(token = %token.token()) ret, err(Debug))]
pub async fn verify_gameserver(
	state: State,
	token: TypedHeader<Authorization<Bearer>>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let claims = state.decode_jwt::<ServerClaims>(token.token())?;

	if claims.expires_at < jwt::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	request.extensions_mut().insert(claims);

	Ok(next.run(request).await)
}
