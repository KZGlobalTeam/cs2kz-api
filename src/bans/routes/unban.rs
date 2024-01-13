use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;

use crate::bans::{CreatedUnban, NewUnban};
use crate::extractors::State;
use crate::responses::Created;
use crate::{responses, Result};

/// Player Unbans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Bans",
  path = "/bans/{ban_id}",
  params(("ban_id" = u32, Path, description = "The ban's ID")),
  request_body = NewUnban,
  responses(
    responses::Created<CreatedUnban>,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans_remove"]),
  ),
)]
pub async fn unban(
	state: State,
	Path(ban_id): Path<u32>,
	Json(unban): Json<NewUnban>,
) -> Result<Created<Json<CreatedUnban>>> {
	let mut transaction = state.transaction().await?;

	let unbanned_by = None::<SteamID>;

	sqlx::query! {
		r#"
		INSERT INTO
		  Unbans (ban_id, reason, unbanned_by)
		VALUES
		  (?, ?, ?)
		"#,
		ban_id,
		unban.reason,
		unbanned_by,
	}
	.execute(transaction.as_mut())
	.await?;

	let unban_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	sqlx::query! {
		r#"
		UPDATE
		  Bans
		SET
		  expires_on = CURRENT_TIMESTAMP()
		WHERE
		  id = ?
		"#,
		ban_id,
	}
	.execute(transaction.as_mut())
	.await?;

	Ok(Created(Json(CreatedUnban { unban_id })))
}
