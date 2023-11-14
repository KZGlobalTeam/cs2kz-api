use {
	crate::{headers::ApiKey, middleware::auth::jwt::GameServerInfo, Error, Result, State},
	axum::{Json, TypedHeader},
	jsonwebtoken as jwt,
	serde::Serialize,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
	/// JWT
	token: String,

	/// Expiration date of the JWT
	expires_on: u64,
}

/// CS2 server authentication.
///
/// This endpoint is used by CS2 game servers to refresh their access token.
#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Auth", context_path = "/api/v0", path = "/auth/token", params(
	("api-key" = u32, Header, description = "API Key"),
), responses(
	(status = 200, body = TokenResponse, description = "The JWT and its expiration date."),
	(status = 401, body = Error, description = "The API Key header was incorrect."),
	(status = 500, body = Error),
))]
pub async fn token(
	state: State,
	TypedHeader(ApiKey(api_key)): TypedHeader<ApiKey>,
) -> Result<Json<TokenResponse>> {
	let server = sqlx::query! {
		r#"
		SELECT
			id,
			ip_address,
			port AS `port: u16`
		FROM
			Servers
		WHERE
			api_key = ?
		"#,
		api_key,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::Unauthorized)?;

	let server_info = GameServerInfo::new(server.id);
	let token = jwt::encode(&state.jwt().header, &server_info, &state.jwt().encode)?;

	Ok(Json(TokenResponse { token, expires_on: server_info.exp }))
}
