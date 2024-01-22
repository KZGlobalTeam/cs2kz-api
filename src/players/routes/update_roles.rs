use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::auth::{Role, RoleFlags};
use crate::extract::State;
use crate::{responses, Result};

/// Overwrites the roles of the specified player.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Players",
  path = "/players/{steam_id}/roles",
  params(SteamID),
  request_body = Vec<Role>,
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn update_roles(
	state: State,
	Path(steam_id): Path<SteamID>,
	Json(roles): Json<Vec<Role>>,
) -> Result<()> {
	let role_flags = RoleFlags::from_iter(roles);

	sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  role_flags = ?
		WHERE
		  steam_id = ?
		"#,
		role_flags,
		steam_id,
	}
	.execute(state.database())
	.await?;

	Ok(())
}
