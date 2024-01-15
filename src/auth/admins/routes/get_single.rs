use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::auth::admins::Admin;
use crate::auth::RoleFlags;
use crate::extractors::State;
use crate::{responses, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/admins/{steam_id}",
  params(SteamID),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(state: State, Path(steam_id): Path<SteamID>) -> Result<Json<Admin>> {
	sqlx::query! {
		r#"
		SELECT
		  p.steam_id `steam_id: SteamID`,
		  p.name,
		  a.role_flags
		FROM
		  Admins a
		  JOIN Players p ON p.steam_id = a.steam_id
		WHERE
		  a.steam_id = ?
		"#,
		steam_id,
	}
	.fetch_optional(state.database())
	.await?
	.map(|row| Admin {
		steam_id: row.steam_id,
		name: row.name,
		roles: RoleFlags(row.role_flags).iter().collect(),
	})
	.map(Json)
	.ok_or(Error::NoContent)
}
