use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::auth::{Role, RoleFlags};
use crate::{audit, responses, AppState, Result};

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
	state: AppState,
	Path(steam_id): Path<SteamID>,
	Json(roles): Json<Vec<Role>>,
) -> Result<()> {
	let role_flags = RoleFlags::from_iter(roles.iter().copied());

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

	audit!("updated roles for user", %steam_id, ?roles);

	Ok(())
}
