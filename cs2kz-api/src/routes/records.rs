use {
	crate::{
		res::{records as res, BadRequest},
		util::Created,
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	cs2kz::{MapIdentifier, Mode, PlayerIdentifier, Runtype, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	utoipa::{IntoParams, ToSchema},
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetRecordsParams<'a> {
	map: Option<MapIdentifier<'a>>,
	stage: Option<u8>,
	course: Option<u8>,
	player: Option<PlayerIdentifier<'a>>,
	mode: Option<Mode>,
	runtype: Option<Runtype>,
	server: Option<ServerIdentifier<'a>>,
	top_only: Option<bool>,
	allow_banned: Option<bool>,
	allow_non_ranked: Option<bool>,

	#[serde(default)]
	offset: u64,
	limit: Option<u64>,
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
		course,
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
) -> Response<Vec<res::Record>> {
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
pub async fn get_record(state: State, Path(record_id): Path<u64>) -> Response<res::Record> {
	sqlx::query! {
		r#"
		SELECT
			r.id,
			m.id map_id,
			m.name map_name,
			c.stage map_stage,
			c.id course_id,
			c.difficulty course_tier,
			f.mode_id,
			r.teleports > 0 `runtype: bool`,
			f.style_id,
			p.name player_name,
			p.id steam_id,
			s.id server_id,
			s.name server_name,
			r.teleports,
			r.time,
			r.created_on
		FROM
			Records r
			JOIN Filters f ON f.id = r.filter_id
			JOIN Courses c ON c.id = f.course_id
			JOIN Maps m ON m.id = c.map_id
			JOIN Players p ON p.id = r.player_id
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
		map_id: record.map_id,
		map_name: record.map_name,
		map_stage: record.map_stage,
		course_id: record.course_id,
		course_tier: record
			.course_tier
			.try_into()
			.expect("found invalid tier"),
		mode: record
			.mode_id
			.try_into()
			.expect("found invalid mode"),
		runtype: record.runtype.into(),
		style: record
			.style_id
			.try_into()
			.expect("found invalid style"),
		player_name: record.player_name,
		steam_id: SteamID::from_id32(record.steam_id).expect("found invalid SteamID"),
		server_id: record.server_id,
		server_name: record.server_name,
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
pub async fn get_replay(state: State, Path(record_id): Path<u32>) -> Response<()> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewRecord {
	course_id: u32,
	steam_id: SteamID,
	filter_id: u32,
	time: f64,
	teleports: u16,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewRecordWithId {
	id: u64,

	#[serde(flatten)]
	record: NewRecord,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Records", context_path = "/api/v0", path = "/records",
	request_body = NewRecord,
	responses(
		(status = 201, body = NewRecordWithId),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_record(
	state: State,
	Json(NewRecord { course_id, steam_id, filter_id, time, teleports }): Json<NewRecord>,
) -> Result<Created<Json<NewRecordWithId>>> {
	// TODO(AlphaKeks): delete this once we have middleware
	let server_id = 0;
	let plugin_version = 0;

	let mut transaction = state.database().begin().await?;

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
		server_id,
		teleports,
		time,
		plugin_version,
	}
	.execute(transaction.as_mut())
	.await?;

	let record_id = sqlx::query!("SELECT MAX(id) id FROM Records")
		.fetch_one(transaction.as_mut())
		.await?
		.id
		.expect("we just inserted a record");

	transaction.commit().await?;

	Ok(Created(Json(NewRecordWithId {
		id: record_id,
		record: NewRecord { course_id, steam_id, filter_id, time, teleports },
	})))
}
