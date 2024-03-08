use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;
use sqlx::{MySql, QueryBuilder, Transaction};

use crate::auth::{Jwt, Server};
use crate::players::models::PlayerUpdateCourseSession;
use crate::players::PlayerUpdate;
use crate::responses::{self, NoContent};
use crate::{AppState, Error, Result};

#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  tag = "Players",
  path = "/players/{steam_id}",
  request_body = PlayerUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn update(
	state: AppState,
	server: Jwt<Server>,
	Path(steam_id): Path<SteamID>,
	Json(params): Json<PlayerUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.begin_transaction().await?;

	update_details(steam_id, &params, &mut transaction).await?;

	sqlx::query! {
		r#"
		INSERT INTO
		  Sessions (
		    player_id,
		    server_id,
		    time_active,
		    time_spectating,
		    time_afk,
		    perfs,
		    bhops_tick0,
		    bhops_tick1,
		    bhops_tick2,
		    bhops_tick3,
		    bhops_tick4,
		    bhops_tick5,
		    bhops_tick6,
		    bhops_tick7,
		    bhops_tick8
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		server.id,
		params.session.time.active.as_secs(),
		params.session.time.spectating.as_secs(),
		params.session.time.afk.as_secs(),
		params.session.bhop_stats.perfs,
		params.session.bhop_stats.tick0,
		params.session.bhop_stats.tick1,
		params.session.bhop_stats.tick2,
		params.session.bhop_stats.tick3,
		params.session.bhop_stats.tick4,
		params.session.bhop_stats.tick5,
		params.session.bhop_stats.tick6,
		params.session.bhop_stats.tick7,
		params.session.bhop_stats.tick8,
	}
	.execute(transaction.as_mut())
	.await?;

	for (course_id, session) in params.course_sessions {
		insert_course_session(steam_id, server.id, course_id, session, &mut transaction).await?;
	}

	transaction.commit().await?;

	Ok(NoContent)
}

async fn update_details(
	steam_id: SteamID,
	params: &PlayerUpdate,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	if params.name.is_none() && params.ip_address.is_none() {
		return Ok(());
	}

	let mut query = QueryBuilder::new("UPDATE Players");
	let mut delimiter = " SET ";

	if let Some(name) = &params.name {
		query.push(delimiter).push(" name = ").push_bind(name);

		delimiter = ",";
	}

	if let Some(ip_address) = params.ip_address {
		query
			.push(delimiter)
			.push(" last_known_ip_address = ")
			.push_bind(ip_address.to_string());
	}

	query.push(" WHERE steam_id = ").push_bind(steam_id);

	let result = query.build().execute(transaction.as_mut()).await?;

	if result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID").with_detail(steam_id));
	}

	Ok(())
}

async fn insert_course_session(
	steam_id: SteamID,
	server_id: u16,
	course_id: u32,
	session: PlayerUpdateCourseSession,
	transaction: &mut Transaction<'static, MySql>,
) -> Result<()> {
	sqlx::query! {
		r#"
		INSERT INTO
		  CourseSessions (
		    player_id,
		    course_id,
		    mode_id,
		    server_id,
		    playtime,
		    total_runs,
		    finished_runs,
		    perfs,
		    bhops_tick0,
		    bhops_tick1,
		    bhops_tick2,
		    bhops_tick3,
		    bhops_tick4,
		    bhops_tick5,
		    bhops_tick6,
		    bhops_tick7,
		    bhops_tick8
		  )
		VALUES
		  (
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?,
		    ?
		  )
		"#,
		steam_id,
		course_id,
		session.mode,
		server_id,
		session.playtime.as_secs(),
		session.total_runs,
		session.finished_runs,
		session.bhop_stats.perfs,
		session.bhop_stats.tick0,
		session.bhop_stats.tick1,
		session.bhop_stats.tick2,
		session.bhop_stats.tick3,
		session.bhop_stats.tick4,
		session.bhop_stats.tick5,
		session.bhop_stats.tick6,
		session.bhop_stats.tick7,
		session.bhop_stats.tick8,
	}
	.execute(transaction.as_mut())
	.await?;

	Ok(())
}
