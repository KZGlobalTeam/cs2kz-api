use axum::Json;

use crate::auth::admins::NewAdmin;
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
	Json(NewAdmin { steam_id, permissions }): Json<NewAdmin>,
) -> Result<Created<()>> {
	sqlx::query! {
		r#"
		INSERT INTO
		  Admins (steam_id, permissions)
		VALUES
		  (?, ?) ON DUPLICATE KEY
		UPDATE
		  permissions = ?
		"#,
		steam_id,
		permissions,
		permissions,
	}
	.fetch_optional(state.database())
	.await?;

	Ok(Created(()))
}
