use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::bans::BanUpdate;
use crate::{audit, responses, AppState, Error, Result};

/// Update an existing ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Bans",
  path = "/bans/{ban_id}",
  params(("ban_id" = u32, Path, description = "The ban's ID")),
  request_body = BanUpdate,
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans"]),
  ),
)]
pub async fn update(
	state: AppState,
	Path(ban_id): Path<u32>,
	Json(ban_update): Json<BanUpdate>,
) -> Result<()> {
	let mut query = QueryBuilder::new("UPDATE Bans");
	let mut delimiter = " SET ";

	if let Some(ref reason) = ban_update.reason {
		query.push(delimiter).push(" reason = ").push_bind(reason);

		delimiter = ",";
	}

	if let Some(ref expires_on) = ban_update.expires_on {
		query
			.push(delimiter)
			.push(" expires_on = ")
			.push_bind(expires_on);
	}

	query.push(" WHERE id = ").push_bind(ban_id);

	let result = query.build().execute(&state.database).await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown_id("ban", ban_id));
	}

	audit!("updated ban", id = %ban_id, update = ?ban_update);

	Ok(())
}
