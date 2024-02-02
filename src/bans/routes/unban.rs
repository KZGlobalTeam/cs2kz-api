use axum::extract::Path;
use axum::Json;

use crate::auth::{Role, Session};
use crate::bans::{CreatedUnban, NewUnban};
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::{audit, responses, AppState, Error, Result};

/// Revert a ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  tag = "Bans",
  path = "/bans/{ban_id}",
  params(("ban_id" = u32, Path, description = "The ban's ID")),
  request_body = NewUnban,
  responses(
    responses::Created<CreatedUnban>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans"]),
  ),
)]
pub async fn unban(
	state: AppState,
	session: Session<{ Role::Bans as u32 }>,
	Path(ban_id): Path<u32>,
	Json(unban): Json<NewUnban>,
) -> Result<Created<Json<CreatedUnban>>> {
	let mut transaction = state.begin_transaction().await?;

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

	audit!("ban invalidated", id = %ban_id);

	sqlx::query! {
		r#"
		INSERT INTO
		  Unbans (ban_id, reason, unbanned_by)
		VALUES
		  (?, ?, ?)
		"#,
		ban_id,
		unban.reason,
		session.user.steam_id,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_foreign_key_violation_of("ban_id") {
			Error::unknown("Ban ID").with_detail(ban_id)
		} else {
			Error::from(err)
		}
	})?;

	let unban_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	audit!("unban created", id = %unban_id, by = %session.user.steam_id, reason = %unban.reason);

	sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  is_banned = false
		WHERE
		  steam_id = (SELECT player_id FROM Bans where id = ?)
		"#,
		ban_id,
	}
	.execute(transaction.as_mut())
	.await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedUnban { unban_id })))
}
