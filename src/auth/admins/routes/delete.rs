use axum::extract::Path;
use cs2kz::SteamID;

use crate::extractors::State;
use crate::sqlx::SqlErrorExt;
use crate::{responses, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Auth",
  path = "/auth/admins/{steam_id}",
  params(SteamID),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["manage"]),
  ),
)]
pub async fn delete(state: State, Path(steam_id): Path<SteamID>) -> Result<()> {
	sqlx::query! {
		r#"
		DELETE FROM
		  Admins
		WHERE
		  steam_id = ?
		"#,
		steam_id,
	}
	.execute(state.database())
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation_of("steam_id") {
			Error::UnknownPlayer { steam_id }
		} else {
			Error::MySql(err)
		}
	})?;

	Ok(())
}
