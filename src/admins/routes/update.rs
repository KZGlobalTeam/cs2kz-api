use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::auth::{Role, RoleFlags};
use crate::responses::{self, NoContent};
use crate::{AppState, Error, Result};

/// Create or update admins.
///
/// Updates are idempotent, so the user's roles will be replaced completely by the roles supplied
/// in the request body.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Admins",
  path = "/admins/{steam_id}",
  params(SteamID),
  request_body = Vec<Role>,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["admin"]),
  ),
)]
pub async fn update(
	state: AppState,
	Path(steam_id): Path<SteamID>,
	Json(roles): Json<Vec<Role>>,
) -> Result<NoContent> {
	let role_flags = roles.into_iter().collect::<RoleFlags>();

	let result = sqlx::query! {
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
	.execute(&state.database)
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID").with_detail(steam_id));
	}

	Ok(NoContent)
}
