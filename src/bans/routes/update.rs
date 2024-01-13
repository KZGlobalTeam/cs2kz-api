use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::bans::BanUpdate;
use crate::extractors::State;
use crate::{responses, Result};

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
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans_edit"]),
  ),
)]
pub async fn update(
	state: State,
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

	query.build().execute(state.database()).await?;

	Ok(())
}
