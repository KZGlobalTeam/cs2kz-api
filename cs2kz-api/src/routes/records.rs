use {
	crate::{
		res::{records as res, BadRequest},
		util::Created,
		Response, Result, State,
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
		(status = 200, body = Vec<Record>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_record(state: State, Path(record_id): Path<u32>) -> Response<Vec<res::Record>> {
	todo!();
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
