use {
	super::{BoundedU64, Created},
	crate::{
		middleware::auth::gameservers::AuthenticatedServer,
		res::{records as res, BadRequest},
		Error, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Extension, Json,
	},
	cs2kz::{MapIdentifier, Mode, PlayerIdentifier, Runtype, ServerIdentifier, SteamID, Style},
	serde::{Deserialize, Serialize},
	utoipa::{IntoParams, ToSchema},
};

/// Query parameters for fetching records.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetRecordsParams<'a> {
	/// A map's ID or name.
	map: Option<MapIdentifier<'a>>,

	/// A map stage.
	stage: Option<u8>,

	/// A course ID.
	course_id: Option<u8>,

	/// A player's `SteamID` or name.
	player: Option<PlayerIdentifier<'a>>,

	/// A mode.
	mode: Option<Mode>,

	/// A runtype (Pro/TP).
	runtype: Option<Runtype>,

	/// A server's ID or name.
	server: Option<ServerIdentifier<'a>>,

	/// Only include personal bests.
	top_only: Option<bool>,

	/// Only include records from (non) banned players.
	allow_banned: Option<bool>,

	/// Only include records on (non) ranked courses.
	allow_non_ranked: Option<bool>,

	#[param(value_type = Option<u64>, default = 0)]
	offset: BoundedU64,

	/// Return at most this many results.
	#[param(value_type = Option<u64>, default = 100, maximum = 500)]
	limit: BoundedU64<100, 500>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Records", context_path = "/api/v0", path = "/records",
	params(GetRecordsParams),
	responses(
		(status = 200, body = Vec<Record>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_records(
	state: State,
	Query(GetRecordsParams {
		map,
		stage,
		course_id,
		player,
		mode,
		runtype,
		server,
		top_only,
		allow_banned,
		allow_non_ranked,
		offset,
		limit,
	}): Query<GetRecordsParams<'_>>,
) -> Result<Json<Vec<res::Record>>> {
	todo!();
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Records", context_path = "/api/v0", path = "/records/{id}",
	params(("id" = u32, Path, description = "The records's ID")),
	responses(
		(status = 200, body = Record),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_record(state: State, Path(record_id): Path<u64>) -> Result<Json<res::Record>> {
	sqlx::query! {
		r#"
		SELECT
			r.id,
			m.id map_id,
			m.name map_name,
			c.id course_id,
			c.map_stage course_stage,
			f.tier course_tier,
			f.mode_id,
			r.teleports > 0 `runtype: bool`,
			r.style_id,
			p.name player_name,
			p.steam_id,
			s.id server_id,
			s.name server_name,
			r.teleports,
			r.time,
			r.created_on
		FROM
			Records r
			JOIN CourseFilters f ON f.id = r.filter_id
			JOIN Courses c ON c.id = f.course_id
			JOIN Maps m ON m.id = c.map_id
			JOIN Players p ON p.steam_id = r.player_id
			JOIN Servers s ON s.id = r.server_id
		WHERE
			r.id = ?
		"#,
		record_id,
	}
	.fetch_optional(state.database())
	.await?
	.map(|record| res::Record {
		id: record.id,
		map: res::RecordMap {
			id: record.map_id,
			name: record.map_name,
			course: res::RecordCourse {
				id: record.course_id,
				stage: record.course_stage,
				tier: record
					.course_tier
					.try_into()
					.expect("found invalid tier"),
			},
		},
		mode: record
			.mode_id
			.try_into()
			.expect("found invalid mode"),
		style: record
			.style_id
			.try_into()
			.expect("found invalid style"),
		player: res::RecordPlayer {
			name: record.player_name,
			steam_id: SteamID::from_id32(record.steam_id).expect("found invalid SteamID"),
		},
		server: res::RecordServer { id: record.server_id, name: record.server_name },
		teleports: record.teleports,
		time: record.time,
		created_on: record.created_on,
	})
	.map(Json)
	.ok_or(Error::NoContent)
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Records", context_path = "/api/v0", path = "/records/{id}/replay",
	params(("id" = u32, Path, description = "The records's ID")),
	responses(
		(status = 200, body = ()),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_replay(state: State, Path(record_id): Path<u32>) -> Result<&'static str> {
	Ok("not yet implemented")
}

/// A newly submitted KZ record.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewRecord {
	/// The ID of the course this record was performed on.
	course_id: u32,

	/// The mode this record was performed in.
	mode: Mode,

	/// The style this record was performed in.
	style: Style,

	/// The `SteamID` of the player who performed this record.
	steam_id: SteamID,

	/// The time it took to finish this run (in seconds).
	time: f64,

	/// The amount of teleports used in this run.
	teleports: u16,

	/// Statistics about how many perfect bhops the player hit during the run.
	bhop_stats: BhopStats,
}

/// Bhop statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BhopStats {
	perfs: u16,
	bhops_tick0: u16,
	bhops_tick1: u16,
	bhops_tick2: u16,
	bhops_tick3: u16,
	bhops_tick4: u16,
	bhops_tick5: u16,
	bhops_tick6: u16,
	bhops_tick7: u16,
	bhops_tick8: u16,
}

/// A newly created KZ record.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedRecord {
	/// The record's ID.
	id: u64,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Records", context_path = "/api/v0", path = "/records",
	request_body = NewRecord,
	responses(
		(status = 201, body = CreatedRecord),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_record(
	state: State,
	Extension(server): Extension<AuthenticatedServer>,
	Json(NewRecord { course_id, mode, style, steam_id, time, teleports, bhop_stats }): Json<
		NewRecord,
	>,
) -> Result<Created<Json<CreatedRecord>>> {
	let filter_id = sqlx::query! {
		r#"
		SELECT
			id
		FROM
			CourseFilters
		WHERE
			course_id = ?
			AND mode_id = ?
			AND has_teleports = ?
		"#,
		course_id,
		mode as u8,
		teleports > 0,
	}
	.fetch_optional(state.database())
	.await?
	.map(|row| row.id)
	.ok_or(Error::MissingFilter)?;

	let mut transaction = state.transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Records (
				filter_id,
				player_id,
				server_id,
				teleports,
				time,
				plugin_version
			)
		VALUES
			(?, ?, ?, ?, ?, ?)
		"#,
		filter_id,
		steam_id.as_u32(),
		server.id,
		teleports,
		time,
		server.plugin_version,
	}
	.execute(transaction.as_mut())
	.await?;

	let record_id = sqlx::query!("SELECT MAX(id) id FROM Records")
		.fetch_one(transaction.as_mut())
		.await?
		.id
		.expect("we just inserted a record");

	transaction.commit().await?;

	Ok(Created(Json(CreatedRecord { id: record_id })))
}
