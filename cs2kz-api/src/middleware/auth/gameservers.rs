use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use jsonwebtoken as jwt;

use super::jwt::GameServerInfo;
use crate::headers::PluginVersion;
use crate::{Error, Result, State};

#[derive(Debug, Clone)]
pub struct AuthenticatedServer {
	pub id: u16,
	pub plugin_version: u16,
}

#[tracing::instrument(skip(state, request, next))]
pub async fn auth_server(
	state: State,
	api_token: TypedHeader<Authorization<Bearer>>,
	plugin_version: TypedHeader<PluginVersion>,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let GameServerInfo { id, exp } = state.jwt().decode(api_token.token())?.claims;

	if exp < jwt::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	let metadata = AuthenticatedServer { id, plugin_version: plugin_version.0.0 };

	request.extensions_mut().insert(metadata);

	Ok(next.run(request).await)
}
