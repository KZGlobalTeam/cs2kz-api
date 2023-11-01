use serde::Serialize;

use {
	crate::{res::BadRequest, Result, State},
	axum::{http::StatusCode, Json},
	cs2kz::SteamID,
	serde::Deserialize,
	std::net::Ipv4Addr,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewPlayer {
	steam_id: SteamID,
	name: String,

	#[schema(value_type = String)]
	ip: Ipv4Addr,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Players", context_path = "/api/v0", path = "/players", request_body = NewPlayer, responses(
	(status = 201, body = NewPlayer),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn root(
	state: State,
	Json(NewPlayer { steam_id, name, ip }): Json<NewPlayer>,
) -> Result<(StatusCode, Json<NewPlayer>)> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (id, name, ip)
		VALUES
			(?, ?, ?)
		"#,
		steam_id.as_u32(),
		name,
		ip.to_string(),
	}
	.execute(state.database())
	.await?;

	Ok((StatusCode::CREATED, Json(NewPlayer { steam_id, name, ip })))
}
