use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use super::jwt::GameServerToken;
use crate::{Result, State};

#[tracing::instrument(skip(state, request, next))]
pub async fn auth_server(
	state: State,
	TypedHeader(token): TypedHeader<Authorization<Bearer>>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let server_data = super::verify_jwt(state, token, |token: &GameServerToken| token.expires_at)?;

	request.extensions_mut().insert(server_data);

	Ok(next.run(request).await)
}
