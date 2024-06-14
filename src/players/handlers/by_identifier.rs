//! HTTP handlers for the `/players/{player}` routes.

use axum::extract::Path;
use axum::Json;
use cs2kz::{PlayerIdentifier, SteamID};
use sqlx::types::Json as SqlJson;
use sqlx::{MySql, QueryBuilder};

use crate::authentication::Jwt;
use crate::authorization::Permissions;
use crate::game_sessions::{CourseSessionID, GameSessionID};
use crate::maps::CourseID;
use crate::openapi::responses::{self, NoContent};
use crate::players::{queries, CourseSession, FullPlayer, PlayerUpdate};
use crate::servers::ServerID;
use crate::sqlx::SqlErrorExt;
use crate::{authentication, authorization, Error, Result, State};

/// Fetch a specific player by their name or SteamID.
///
/// The object returned from this endpoint will include an `ip_address` field if and only if the
/// requesting user is authorized to manage bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<FullPlayer>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: State,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
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
		.ok_or_else(|| Error::not_found("player"))?;

	// Filter out IP address if we're not in a test and the user does not have permission to
	// view IP addresses
	if cfg!(not(test)) && session.is_none() {
		player.ip_address = None;
	}

	Ok(Json(player))
}

/// Update an existing player.
///
/// This endpoint is for CS2 servers. Whenever a player disconnects, or when the map changes, they
/// will update players using this endpoint.
#[tracing::instrument(skip(state))]
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
  ),
)]
pub async fn patch(
	state: State,
	Jwt {
		payload: server, ..
	}: Jwt<authentication::Server>,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate {
		name,
		ip_address,
		session,
		preferences,
	}): Json<PlayerUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  name = ?,
		  ip_address = ?,
		  preferences = ?
		WHERE
		  id = ?
		"#,
		name,
		ip_address,
		SqlJson(&preferences),
		steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	match query_result.rows_affected() {
		0 => return Err(Error::not_found("player")),
		n => assert_eq!(n, 1, "updated more than 1 player"),
	}

	tracing::trace!(target: "cs2kz_api::audit_log", "updated player");

	let session_id: GameSessionID = sqlx::query! {
		r#"
		INSERT INTO
		  GameSessions (
		    player_id,
		    server_id,
		    time_active,
		    time_spectating,
		    time_afk,
		    bhops,
		    perfs
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		server.id(),
		session.time_spent.active.as_secs(),
		session.time_spent.spectating.as_secs(),
		session.time_spent.afk.as_secs(),
		session.bhop_stats.bhops,
		session.bhop_stats.perfs,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::not_found("player").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	tracing::trace!(target: "cs2kz_api::audit_log", %session_id, "created game session");

	let mut course_session_ids = Vec::with_capacity(session.course_sessions.len());

	for (course_id, course_session) in session.course_sessions {
		course_session_ids.push(
			insert_course_session(
				steam_id,
				server.id(),
				course_id,
				course_session,
				&mut transaction,
			)
			.await?,
		);
	}

	tracing::trace!(target: "cs2kz_api::audit_log", ?course_session_ids, "created course sessions");

	transaction.commit().await?;

	Ok(NoContent)
}

/// Inserts a [`CourseSession`] into the database and returns the generated [`CourseSessionID`].
async fn insert_course_session(
	steam_id: SteamID,
	server_id: ServerID,
	course_id: CourseID,
	CourseSession {
		mode,
		playtime,
		started_runs,
		finished_runs,
		bhop_stats,
	}: CourseSession,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<CourseSessionID> {
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
		    bhops,
		    perfs
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		course_id,
		mode,
		server_id,
		playtime,
		started_runs,
		finished_runs,
		bhop_stats.bhops,
		bhop_stats.perfs,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::not_found("player").context(err)
		} else if err.is_fk_violation_of("course_id") {
			Error::not_found("course").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	tracing::trace! {
		target: "cs2kz_api::audit_log",
		%course_id,
		%session_id,
		"created course session",
	};

	Ok(session_id)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::net::{IpAddr, Ipv4Addr};
	use std::time::Duration;

	use serde_json::{json, Value as JsonValue};
	use uuid::Uuid;

	use crate::game_sessions::TimeSpent;
	use crate::players::{FullPlayer, PlayerUpdate, Session};
	use crate::records::BhopStats;

	#[crate::integration_test]
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

	#[crate::integration_test]
	async fn update_player(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/players/alphakeks"))
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let player = response.json::<FullPlayer>().await?;
		let new_name = player.name.chars().rev().collect::<String>();
		let new_ip = Ipv4Addr::new(69, 69, 69, 69).into();

		let update = PlayerUpdate {
			name: new_name.clone(),
			ip_address: new_ip,
			session: Session {
				time_spent: TimeSpent {
					active: Duration::from_secs(6942).into(),
					spectating: Duration::from_secs(1337).into(),
					afk: Duration::from_secs(0).into(),
				},
				bhop_stats: BhopStats {
					bhops: 13847,
					perfs: 6237,
				},
				course_sessions: BTreeMap::new(),
			},
			preferences: json!({ "funny_test": ctx.test_id }),
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

		let actual_ip = sqlx::query_scalar! {
			r#"
			SELECT
			  ip_address `ip: IpAddr`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			player.steam_id,
		}
		.fetch_one(&ctx.database)
		.await?;

		match (new_ip, actual_ip) {
			(IpAddr::V4(new), IpAddr::V4(actual)) => {
				assert_eq!(new, actual);
			}
			(IpAddr::V6(new), IpAddr::V6(actual)) => {
				assert_eq!(new, actual);
			}
			(IpAddr::V4(new), IpAddr::V6(actual)) => {
				assert_eq!(new.to_ipv6_mapped(), actual);
			}
			(IpAddr::V6(new), IpAddr::V4(actual)) => {
				assert_eq!(new, actual.to_ipv6_mapped());
			}
		}

		let url = ctx.url(format_args!("/players/{}", player.steam_id));
		let response = ctx.http_client.get(url).send().await?;

		assert_eq!(response.status(), 200);

		let player = response.json::<FullPlayer>().await?;

		assert_eq!(player.name, new_name);

		let url = ctx.url(format_args!("/players/{}/preferences", player.steam_id));
		let response = ctx.http_client.get(url).send().await?;

		assert_eq!(response.status(), 200);

		let mut preferences = response.json::<JsonValue>().await?;
		let funny_test = preferences
			.get_mut("funny_test")
			.map(JsonValue::take)
			.map(serde_json::from_value::<Uuid>)
			.expect("this cannot fail")?;

		assert_eq!(funny_test, ctx.test_id);
	}
}
