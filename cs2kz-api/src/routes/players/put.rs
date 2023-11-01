use {
	crate::{res::BadRequest, Result, State},
	axum::{extract::Path, Json},
	cs2kz::SteamID,
	serde::Deserialize,
	std::net::Ipv4Addr,
	utoipa::ToSchema,
};

#[derive(Debug, Deserialize, ToSchema)]
#[rustfmt::skip]
pub struct PlayerUpdate {
	/// The player's new name.
	name: String,

	/* TODO
	 *
	 * /// The additional playtime recorded by the server.
	 * playtime: u32,
	 *
	 */

	/// The player's new IP address.
	#[schema(value_type = String)]
	ip: Ipv4Addr,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Players", context_path = "/api/v0", path = "/players/{steam_id}", request_body = PlayerUpdate, params(
	("steam_id" = SteamID, Path, description = "The player's SteamID or name")
), responses(
	(status = 200),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn steam_id(
	state: State,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate { name, ip }): Json<PlayerUpdate>,
) -> Result<()> {
	sqlx::query! {
		r#"
		UPDATE
			Players
		SET
			name = ?,
			ip = ?
		WHERE
			id = ?
		"#,
		name,
		ip.to_string(),
		steam_id.as_u32(),
	}
	.execute(state.database())
	.await?;

	Ok(())
}
