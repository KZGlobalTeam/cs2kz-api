use {
	super::jwt::GameServerInfo,
	crate::{middleware, state::JwtState, Error, Result, State},
	axum::{extract::Request, middleware::Next, response::Response},
	axum_extra::{
		headers::{authorization::Bearer, Authorization},
		TypedHeader,
	},
	jsonwebtoken as jwt,
	serde::Deserialize,
};

#[derive(Debug, Deserialize)]
struct ServerMetadata {
	plugin_version: u16,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedServer {
	pub id: u16,
	pub plugin_version: u16,
}

#[tracing::instrument(skip(state, request, next))]
pub async fn auth_server(
	state: State,
	api_token: TypedHeader<Authorization<Bearer>>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let JwtState { decode, validation, .. } = state.jwt();
	let GameServerInfo { id, exp } = jwt::decode(api_token.token(), decode, validation)?.claims;

	if exp < jwt::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	let (metadata, mut request) = middleware::deserialize_body::<ServerMetadata>(request).await?;

	let Some(ServerMetadata { plugin_version }) = metadata else {
		return Err(Error::InvalidRequestBody);
	};

	request
		.extensions_mut()
		.insert(AuthenticatedServer { id, plugin_version });

	Ok(next.run(request).await)
}
