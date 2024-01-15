use axum::Json;

use crate::auth::admins::NewAdmin;
use crate::auth::RoleFlags;
use crate::extractors::State;
use crate::responses::Created;
use crate::{responses, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Auth",
  path = "/auth/admins",
  request_body = NewAdmin,
  responses(
    responses::Created<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["manage_admins"]),
  ),
)]
pub async fn update(
	state: State,
	Json(NewAdmin { steam_id, roles }): Json<NewAdmin>,
) -> Result<Created<()>> {
	let role_flags = roles.into_iter().collect::<RoleFlags>();

	sqlx::query! {
		r#"
		INSERT INTO
		  Admins (steam_id, role_flags)
		VALUES
		  (?, ?) ON DUPLICATE KEY
		UPDATE
		  role_flags = ?
		"#,
		steam_id,
		role_flags,
		role_flags,
	}
	.fetch_optional(state.database())
	.await?;

	Ok(Created(()))
}
