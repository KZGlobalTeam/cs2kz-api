use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::Extension;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;

use super::jwt::WebUser;
use crate::{Result, State};

#[tracing::instrument(skip(state, request, next))]
pub async fn auth_web_user(
	state: State,
	TypedHeader(api_token): TypedHeader<Authorization<Bearer>>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let user_data = super::verify_jwt(state, api_token, |token: &WebUser| token.expires_at)?;

	request.extensions_mut().insert(user_data);

	Ok(next.run(request).await)
}

#[allow(unused)]
#[tracing::instrument(skip(state, request, next))]
pub async fn auth_admin(
	state: State,
	Extension(user_data): Extension<WebUser>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	unimplemented!("make sure the user has correct permissions");

	Ok(next.run(request).await)
}
