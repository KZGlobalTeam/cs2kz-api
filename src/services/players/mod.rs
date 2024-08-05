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
			  id = COALESCE(?, id)
			  AND name LIKE COALESCE(?, name)
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

#[cfg(test)]
mod tests
{
	use std::iter;

	use color_eyre::eyre::ContextCompat;
	use cs2kz::SteamID;
	use fake::{Fake, Faker};
	use serde_json::json;
	use sqlx::{MySql, Pool};

	use super::*;
	use crate::testing;

	const ALPHAKEKS_ID: SteamID = match SteamID::new(76561198282622073_u64) {
		Some(id) => id,
		None => unreachable!(),
	};

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_player_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayerRequest { identifier: ALPHAKEKS_ID.into() };
		let res = svc.fetch_player(req).await?.context("got `None`")?;

		testing::assert_eq!(res.info.name, "AlphaKeks");
		testing::assert_eq!(res.info.steam_id, ALPHAKEKS_ID);
		testing::assert!(!res.is_banned);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_player_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayerRequest { identifier: "foobar".parse()? };
		let res = svc.fetch_player(req).await?;

		testing::assert!(res.is_none());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/players.sql")
	)]
	async fn fetch_players_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayersRequest { limit: Default::default(), offset: Default::default() };
		let res = svc.fetch_players(req).await?;

		testing::assert_eq!(res.players.len(), 4);
		testing::assert_eq!(res.total, 4);

		for found in ["AlphaKeks", "iBrahizy", "zer0.k", "GameChaos"]
			.iter()
			.map(|name| res.players.iter().find(|p| &p.info.name == name))
		{
			testing::assert!(found.is_some());
		}

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/players.sql")
	)]
	async fn fetch_players_works_with_limit(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayersRequest { limit: 2.into(), offset: Default::default() };
		let res = svc.fetch_players(req).await?;

		testing::assert_eq!(res.players.len(), 2);
		testing::assert_eq!(res.total, 4);

		let found = ["AlphaKeks", "iBrahizy", "zer0.k", "GameChaos"]
			.iter()
			.filter_map(|name| res.players.iter().find(|p| &p.info.name == name))
			.count();

		testing::assert_eq!(found, 2);

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/players.sql")
	)]
	async fn fetch_players_works_with_offset(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayersRequest { limit: Default::default(), offset: Default::default() };
		let all = svc.fetch_players(req).await?;

		testing::assert_eq!(all.players.len() as u64, all.total);

		let req = FetchPlayersRequest { limit: 2.into(), offset: 0.into() };
		let first_two = svc.fetch_players(req).await?;

		testing::assert_eq!(first_two.players.len(), 2);
		testing::assert_eq!(first_two.total, 4);

		let req = FetchPlayersRequest { limit: 2.into(), offset: 2.into() };
		let last_two = svc.fetch_players(req).await?;

		testing::assert_eq!(first_two.players.len(), 2);
		testing::assert_eq!(first_two.total, 4);

		let all = all.players.into_iter();
		let chained = first_two.players.into_iter().chain(last_two.players);

		for (a, b) in iter::zip(all, chained) {
			testing::assert_eq!(a, b);
		}

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/player-preferences.sql")
	)]
	async fn fetch_player_preferences_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayerPreferencesRequest { identifier: ALPHAKEKS_ID.into() };
		let res = svc
			.fetch_player_preferences(req)
			.await?
			.context("got `None`")?;

		testing::assert_eq!(res.preferences, json!({ "foo": "bar" }));

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn fetch_player_preferences_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);
		let req = FetchPlayerPreferencesRequest { identifier: "foobar".parse()? };
		let res = svc.fetch_player_preferences(req).await?;

		testing::assert!(res.is_none());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn register_player_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);

		let steam_id = const {
			match SteamID::new(76561198264939817) {
				Some(id) => id,
				None => unreachable!(),
			}
		};

		let req = RegisterPlayerRequest {
			name: String::from("iBrahizy"),
			steam_id,
			ip_address: "::1".parse()?,
		};

		let res = svc.register_player(req).await?;

		testing::assert_eq!(res.player_id, steam_id);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn register_player_already_exists(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);

		let req = RegisterPlayerRequest {
			name: String::from("AlphaKeks"),
			steam_id: ALPHAKEKS_ID,
			ip_address: "::1".parse()?,
		};

		let res = svc.register_player(req).await.unwrap_err();

		testing::assert_matches!(res, Error::PlayerAlreadyExists);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_player_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);

		let req = UpdatePlayerRequest {
			player_id: ALPHAKEKS_ID,
			server_id: 1.into(),
			name: String::from("(͡ ͡° ͜ つ ͡͡°)"),
			ip_address: "::1".parse()?,
			preferences: json!({ "foo": "bar" }),
			session: Faker.fake(),
		};

		let res = svc.update_player(req).await?;

		testing::assert!(res.course_session_ids.is_empty());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_player_fails_player_does_not_exist(
		database: Pool<MySql>,
	) -> color_eyre::Result<()>
	{
		let svc = testing::player_svc(database);

		let steam_id = const {
			match SteamID::new(76561198264939817) {
				Some(id) => id,
				None => unreachable!(),
			}
		};

		let req = UpdatePlayerRequest {
			player_id: steam_id,
			server_id: 1.into(),
			name: String::from("(͡ ͡° ͜ つ ͡͡°)"),
			ip_address: "::1".parse()?,
			preferences: json!({ "foo": "bar" }),
			session: Faker.fake(),
		};

		let res = svc.update_player(req).await.unwrap_err();

		testing::assert_matches!(res, Error::PlayerDoesNotExist);

		Ok(())
	}
}
