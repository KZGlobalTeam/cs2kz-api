//! Handlers for the `/players/{player}` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::{PlayerIdentifier, SteamID};
use sqlx::{MySql, QueryBuilder};
use tracing::trace;

use crate::auth::{self, Jwt, RoleFlags};
use crate::maps::CourseID;
use crate::players::{queries, CourseSession, FullPlayer, PlayerUpdate};
use crate::responses::{self, NoContent};
use crate::servers::ServerID;
use crate::sqlx::SqlErrorExt;
use crate::{Error, Result, State};

/// Fetch a specific player.
///
/// If you send a cookie that shows you're "logged in", and you happen to have permissions for
/// managing bans, the response will include the player's IP address.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<FullPlayer>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
	session: Option<auth::Session<auth::HasRoles<{ RoleFlags::BANS.value() }>>>,
	Path(player): Path<PlayerIdentifier>,
) -> Result<Json<FullPlayer>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push(" p.id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push(" p.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let mut player = query
		.build_query_as::<FullPlayer>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	if session.is_none() {
		player.ip_address = None;
	}

	Ok(Json(player))
}

/// Updates information about a player.
///
/// This endpoint will be hit periodically by CS2 servers whenever a map changes, or a player
/// disconnects.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  path = "/players/{steam_id}",
  tag = "Players",
  security(("CS2 Server" = [])),
  params(SteamID),
  request_body = PlayerUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn patch(
	state: &State,
	Jwt { payload: server, .. }: Jwt<auth::Server>,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate { name, ip_address, session }): Json<PlayerUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  name = ?,
		  ip_address = ?
		WHERE
		  id = ?
		"#,
		name,
		ip_address.to_string(),
		steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID"));
	}

	trace!(target: "audit_log", %steam_id, "updated player");

	let session_id = sqlx::query! {
		r#"
		INSERT INTO
		  GameSessions (
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
		server.id(),
		session.time_spent.active.as_secs(),
		session.time_spent.spectating.as_secs(),
		session.time_spent.afk.as_secs(),
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
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::unknown("player").with_source(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id();

	trace!(target: "audit_log", %steam_id, session.id = %session_id, "created game session");

	for (course_id, course_session) in session.course_sessions {
		insert_course_session(steam_id, server.id(), course_id, course_session, &mut transaction)
			.await?;
	}

	transaction.commit().await?;

	Ok(NoContent)
}

/// Inserts course sessions into the database.
async fn insert_course_session(
	steam_id: SteamID,
	server_id: ServerID,
	course_id: CourseID,
	CourseSession { mode, playtime, started_runs, finished_runs, bhop_stats }: CourseSession,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<()> {
	let session_id = sqlx::query! {
		r#"
		INSERT INTO
		  CourseSessions (
		    player_id,
		    course_id,
		    mode_id,
		    server_id,
		    playtime,
		    started_runs,
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
		  (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		course_id,
		mode,
		server_id,
		playtime.as_secs(),
		started_runs,
		finished_runs,
		bhop_stats.perfs,
		bhop_stats.tick0,
		bhop_stats.tick1,
		bhop_stats.tick2,
		bhop_stats.tick3,
		bhop_stats.tick4,
		bhop_stats.tick5,
		bhop_stats.tick6,
		bhop_stats.tick7,
		bhop_stats.tick8,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::unknown("player").with_source(err)
		} else if err.is_fk_violation_of("course_id") {
			Error::unknown("course").with_source(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id();

	trace! {
		target: "audit_log",
		%steam_id,
		course.id = %course_id,
		session.id = %session_id,
		"created course session",
	};

	Ok(())
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::net::Ipv4Addr;
	use std::time::Duration;

	use crate::game_sessions::TimeSpent;
	use crate::players::{FullPlayer, PlayerUpdate, Session};
	use crate::records::BhopStats;

	#[crate::test]
	async fn fetch_player(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/players/alphakeks"))
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let alphakeks = response.json::<FullPlayer>().await?;

		assert_eq!(alphakeks.name, "AlphaKeks");
		assert_eq!(alphakeks.steam_id, 76561198282622073_u64);
	}

	#[crate::test]
	async fn update_player(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/players/alphakeks"))
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let player = response.json::<FullPlayer>().await?;
		let new_name = player.name.chars().rev().collect::<String>();

		let update = PlayerUpdate {
			name: new_name.clone(),
			ip_address: Ipv4Addr::new(0, 0, 0, 0),
			session: Session {
				time_spent: TimeSpent {
					active: Duration::from_secs(6942).into(),
					spectating: Duration::from_secs(1337).into(),
					afk: Duration::from_secs(0).into(),
				},
				bhop_stats: BhopStats {
					perfs: 6237,
					tick0: 1195,
					tick1: 2787,
					tick2: 2002,
					tick3: 9782,
					tick4: 2454,
					tick5: 5859,
					tick6: 1782,
					tick7: 1355,
					tick8: 2365,
				},
				course_sessions: BTreeMap::new(),
			},
		};

		let url = ctx.url(format_args!("/players/{}", player.steam_id));
		let jwt = ctx.auth_server(Duration::from_secs(60 * 60))?;

		let response = ctx
			.http_client
			.patch(url)
			.header("Authorization", format!("Bearer {jwt}"))
			.json(&update)
			.send()
			.await?;

		assert_eq!(response.status(), 204);

		let url = ctx.url(format_args!("/players/{}", player.steam_id));
		let response = ctx.http_client.get(url).send().await?;

		assert_eq!(response.status(), 200);

		let player = response.json::<FullPlayer>().await?;

		assert_eq!(player.name, new_name);
	}
}
