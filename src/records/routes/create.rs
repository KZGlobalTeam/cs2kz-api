use axum::Json;

use crate::auth::{Jwt, Server};
use crate::records::{CreatedRecord, NewRecord};
use crate::responses::{self, Created};
use crate::{query, AppState, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Records",
  path = "/records",
  responses(
    responses::Created<CreatedRecord>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn create(
	state: AppState,
	server: Jwt<Server>,
	Json(record): Json<NewRecord>,
) -> Result<Created<Json<CreatedRecord>>> {
	let mut transaction = state.begin_transaction().await?;

	let filter_id = sqlx::query! {
		r#"
		SELECT
		  id
		FROM
		  CourseFilters
		WHERE
		  course_id = ?
		  AND mode_id = ?
		  AND teleports = ?
		"#,
		record.course_id,
		record.mode,
		record.teleports > 0,
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| row.id)
	.ok_or(Error::invalid("course ID").with_detail(record.course_id))?;

	sqlx::query! {
		r#"
		INSERT INTO
		  Records (
		    player_id,
		    filter_id,
		    style_id,
		    teleports,
		    time,
		    server_id,
		    perfs,
		    bhops_tick0,
		    bhops_tick1,
		    bhops_tick2,
		    bhops_tick3,
		    bhops_tick4,
		    bhops_tick5,
		    bhops_tick6,
		    bhops_tick7,
		    bhops_tick8,
		    plugin_version_id
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		record.steam_id,
		filter_id,
		record.style,
		record.teleports,
		record.time,
		server.id,
		record.bhop_stats.perfs,
		record.bhop_stats.tick0,
		record.bhop_stats.tick1,
		record.bhop_stats.tick2,
		record.bhop_stats.tick3,
		record.bhop_stats.tick4,
		record.bhop_stats.tick5,
		record.bhop_stats.tick6,
		record.bhop_stats.tick7,
		record.bhop_stats.tick8,
		server.plugin_version_id,
	}
	.execute(transaction.as_mut())
	.await?;

	let record_id = query::last_insert_id::<u64>(transaction.as_mut()).await?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedRecord { record_id })))
}
