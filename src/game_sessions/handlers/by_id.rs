//! Handlers for the `/sessions/{session_id}` routes.

use std::num::NonZeroU64;

use axum::extract::Path;
use axum::Json;

use crate::game_sessions::GameSession;
use crate::{responses, AppState, Error, Result};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/sessions/{session_id}",
  tag = "Sessions",
  params(("sesion_id" = u64, Path, description = "The session's ID")),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: AppState, Path(session_id): Path<NonZeroU64>) -> Result<Json<GameSession>> {
	let session = sqlx::query_as(
		r#"
		SELECT
		  s.id,
		  p.name player_name,
		  p.id player_id,
		  sv.name server_name,
		  sv.id server_id,
		  s.time_active,
		  s.time_spectating,
		  s.time_afk,
		  s.perfs,
		  s.bhops_tick0,
		  s.bhops_tick1,
		  s.bhops_tick2,
		  s.bhops_tick3,
		  s.bhops_tick4,
		  s.bhops_tick5,
		  s.bhops_tick6,
		  s.bhops_tick7,
		  s.bhops_tick8,
		  s.created_on
		FROM
		  GameSessions s
		  JOIN Players p ON p.id = s.player_id
		  JOIN Servers sv ON sv.id = s.server_id
		WHERE
		  s.id = ?
		"#,
	)
	.bind(session_id.get())
	.fetch_optional(&state.database)
	.await?
	.ok_or(Error::no_content())?;

	Ok(Json(session))
}
