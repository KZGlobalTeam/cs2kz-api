//! A service for managing KZ Players.

use std::{fmt, iter};

use axum::extract::FromRef;
use sqlx::{MySql, Pool};
use tap::Conv;

use crate::database::{SqlErrorExt, TransactionExt};
use crate::services::{AuthService, SteamService};

pub(crate) mod http;
mod queries;

mod error;
pub use error::{Error, Result};

pub(crate) mod models;
pub use models::{
	CourseSession,
	CourseSessionData,
	CourseSessionID,
	CourseSessionIter,
	FetchPlayerPreferencesRequest,
	FetchPlayerPreferencesResponse,
	FetchPlayerRequest,
	FetchPlayerResponse,
	FetchPlayersRequest,
	FetchPlayersResponse,
	FetchSteamProfileResponse,
	PlayerInfo,
	RegisterPlayerRequest,
	RegisterPlayerResponse,
	Session,
	SessionID,
	UpdatePlayerRequest,
	UpdatePlayerResponse,
};

/// A service for managing KZ Players.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct PlayerService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
	steam_svc: SteamService,
}

impl fmt::Debug for PlayerService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("PlayerService").finish_non_exhaustive()
	}
}

impl PlayerService
{
	/// Create a new [`PlayerService`].
	#[tracing::instrument]
	pub fn new(database: Pool<MySql>, auth_svc: AuthService, steam_svc: SteamService) -> Self
	{
		Self { database, auth_svc, steam_svc }
	}

	/// Fetches a single player.
	///
	/// This will return `Ok(None)` if the player was not found, but everything
	/// else went fine.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_player(&self, req: FetchPlayerRequest)
	-> Result<Option<FetchPlayerResponse>>
	{
		let res = sqlx::query_as(
			r"
			SELECT
			  SQL_CALC_FOUND_ROWS p.id player_id,
			  p.name player_name,
			  p.ip_address,
			  (
			    SELECT
			      COUNT(b.id)
			    FROM
			      Bans b
			    WHERE
			      b.player_id = p.id
			      AND b.expires_on > NOW()
			  ) is_banned
			FROM
			  Players p
			WHERE
			  p.id = COALESCE(?, p.id)
			  AND p.name LIKE COALESCE(?, p.name)
			LIMIT
			  1
			",
		)
		.bind(req.identifier.as_id())
		.bind(req.identifier.as_name().map(|name| format!("%{name}%")))
		.fetch_optional(&self.database)
		.await?;

		Ok(res)
	}

	/// Fetches potentially many players.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_players(&self, req: FetchPlayersRequest) -> Result<FetchPlayersResponse>
	{
		let mut txn = self.database.begin().await?;

		let players = sqlx::query_as::<_, FetchPlayerResponse>(&format!(
			r"
			{}
			LIMIT
			  ? OFFSET ?
			",
			queries::SELECT,
		))
		.bind(*req.limit)
		.bind(*req.offset)
		.fetch_all(txn.as_mut())
		.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchPlayersResponse { players, total })
	}

	/// Fetches a player's in-game preferences.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn fetch_player_preferences(
		&self,
		req: FetchPlayerPreferencesRequest,
	) -> Result<Option<FetchPlayerPreferencesResponse>>
	{
		let res = sqlx::query_scalar::<_, serde_json::Value>(
			r"
			SELECT
			  preferences
			FROM
			  Players
			WHERE
			  p.id = COALESCE(?, p.id)
			  AND p.name LIKE COALESCE(?, p.name)
			LIMIT
			  1
			",
		)
		.bind(req.identifier.as_id())
		.bind(req.identifier.as_name().map(|name| format!("%{name}%")))
		.fetch_optional(&self.database)
		.await?
		.map(|preferences| FetchPlayerPreferencesResponse { preferences });

		Ok(res)
	}

	/// Registers a new player.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn register_player(
		&self,
		req: RegisterPlayerRequest,
	) -> Result<RegisterPlayerResponse>
	{
		sqlx::query! {
			r"
			INSERT INTO
			  Players (id, name, ip_address)
			VALUES
			  (?, ?, ?)
			",
			req.steam_id,
			req.name,
			req.ip_address,
		}
		.execute(&self.database)
		.await
		.map_err(|error| match error.is_duplicate_entry() {
			true => Error::PlayerAlreadyExists,
			false => Error::Database(error),
		})?;

		tracing::info!(target: "cs2kz_api::audit_log", "registered new player");

		Ok(RegisterPlayerResponse { player_id: req.steam_id })
	}

	/// Updates an existing player.
	#[tracing::instrument(level = "debug", err(Debug, level = "debug"))]
	pub async fn update_player(&self, req: UpdatePlayerRequest) -> Result<UpdatePlayerResponse>
	{
		let mut txn = self.database.begin().await?;

		let update_result = sqlx::query! {
			r"
			UPDATE
			  Players
			SET
			  name = ?,
			  ip_address = ?,
			  preferences = ?,
			  last_seen_on = NOW()
			WHERE
			  id = ?
			",
			req.name,
			req.ip_address,
			req.preferences,
			req.player_id,
		}
		.execute(txn.as_mut())
		.await?;

		match update_result.rows_affected() {
			0 => return Err(Error::PlayerDoesNotExist),
			n => assert_eq!(n, 1, "updated more than one player"),
		}

		tracing::info!(target: "cs2kz_api::audit_log", "updated player");

		let session_id = sqlx::query! {
			r"
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
			",
			req.player_id,
			req.server_id,
			req.session.seconds_active,
			req.session.seconds_spectating,
			req.session.seconds_afk,
			req.session.bhop_stats.total,
			req.session.bhop_stats.perfs,
		}
		.execute(txn.as_mut())
		.await?
		.last_insert_id()
		.conv::<SessionID>();

		tracing::info! {
			target: "cs2kz_api::audit_log",
			%session_id,
			"recorded in-game session",
		};

		let mut course_session_ids = Vec::with_capacity(req.session.course_sessions.len() * 2);

		for (course_id, (mode, session_data)) in req
			.session
			.course_sessions
			.iter()
			.flat_map(|(&course_id, session)| iter::zip(iter::repeat(course_id), session))
		{
			let course_session_id = sqlx::query! {
				r"
				INSERT INTO
				  CourseSessions (
				    player_id,
				    course_id,
				    mode,
				    server_id,
				    playtime,
				    started_runs,
				    finished_runs,
				    bhops,
				    perfs
				  )
				VALUES
				  (?, ?, ?, ?, ?, ?, ?, ?, ?)
				",
				req.player_id,
				course_id,
				mode,
				req.server_id,
				session_data.playtime,
				session_data.started_runs,
				session_data.finished_runs,
				session_data.bhop_stats.total,
				session_data.bhop_stats.perfs,
			}
			.execute(txn.as_mut())
			.await?
			.last_insert_id()
			.conv::<CourseSessionID>();

			tracing::info! {
				target: "cs2kz_api::audit_log",
				%course_id,
				%mode,
				%course_session_id,
				"recorded course session",
			};

			course_session_ids.push(course_session_id);
		}

		course_session_ids.sort_unstable();
		txn.commit().await?;

		Ok(UpdatePlayerResponse { session_id, course_session_ids })
	}
}
