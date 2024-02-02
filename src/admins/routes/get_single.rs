use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::admins::Admin;
use crate::auth::RoleFlags;
use crate::{responses, AppState, Error, Result};

/// Fetch a specific user with elevated permissions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Admins",
  path = "/admins/{steam_id}",
  params(SteamID),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_single(state: AppState, Path(steam_id): Path<SteamID>) -> Result<Json<Admin>> {
	sqlx::query! {
		r#"
		SELECT
		  name,
		  steam_id `steam_id: SteamID`,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Players
		WHERE
		  steam_id = ?
		"#,
		steam_id,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| Admin {
		name: row.name,
		steam_id: row.steam_id,
		roles: row.role_flags.iter().collect(),
	})
	.map(Json)
	.ok_or(Error::no_data())
}
