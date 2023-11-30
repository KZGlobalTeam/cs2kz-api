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
	TypedHeader(api_token): TypedHeader<Authorization<Bearer>>,
	request: Request,
	next: Next,
) -> Result<Response> {
	let JwtState { decode, validation, .. } = &state.jwt;
	let server_info = jwt::decode::<GameServerInfo>(api_token.token(), decode, validation)?.claims;

	if server_info.exp < jwt::get_current_timestamp() {
		return Err(Error::Unauthorized);
	}

	let (metadata, mut request) = middleware::deserialize_body::<ServerMetadata>(request).await?;

	request
		.extensions_mut()
		.insert(AuthenticatedServer {
			id: server_info.id,
			plugin_version: metadata.plugin_version,
		});

	Ok(next.run(request).await)
}
