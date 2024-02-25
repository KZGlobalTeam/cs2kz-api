use axum::extract::Path;
use axum::Json;

use crate::auth::sessions::Admin;
use crate::auth::{Role, Session};
use crate::bans::{CreatedUnban, NewUnban};
use crate::responses::Created;
use crate::sqlx::SqlErrorExt;
use crate::{audit, query, responses, AppState, Error, Result};

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
	session: Session<Admin<{ Role::Bans as u32 }>>,
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
			Error::unknown_id("ban", ban_id)
		} else {
			Error::from(err)
		}
	})?;

	let unban_id = query::last_insert_id::<u32>(transaction.as_mut()).await?;

	transaction.commit().await?;

	audit!("unban created", id = %unban_id, by = %session.user.steam_id, reason = ?unban.reason);

	Ok(Created(Json(CreatedUnban { ban_id, unban_id })))
}
