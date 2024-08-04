//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::{Mode, SteamID, Styles};
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	Error,
	FetchRecordRequest,
	FetchRecordResponse,
	FetchRecordsRequest,
	FetchRecordsResponse,
	FetchReplayRequest,
	FetchReplayResponse,
	RecordID,
	RecordService,
	SubmitRecordRequest,
	SubmitRecordResponse,
	UpdateRecordAction,
	UpdateRecordRequest,
	UpdateRecordResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::jwt::{self, JwtLayer};
use crate::services::auth::session::user::Permissions;
use crate::services::auth::session::{authorization, SessionManagerLayer};
use crate::services::auth::{Jwt, Session};
use crate::services::maps::CourseID;
use crate::stats::BhopStats;
use crate::time::Seconds;

impl From<RecordService> for Router
{
	fn from(svc: RecordService) -> Self
	{
		let jwt_auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(JwtLayer::<jwt::ServerInfo>::new(svc.auth_svc.clone()));

		let session_auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::RequiredPermissions(Permissions::RECORDS),
			));

		let no_cors = Router::new()
			.route("/", routing::post(submit_record).layer(jwt_auth))
			.with_state(svc.clone());

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:record", routing::get(get_single))
			.route("/:record/replay", routing::get(get_replay))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/:record", routing::patch(update_record).layer(session_auth))
			.route_layer(middleware::cors::dashboard([http::Method::OPTIONS, http::Method::PATCH]))
			.with_state(svc.clone());

		no_cors.merge(public).merge(protected)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/records", tag = "Records", params(FetchRecordsRequest))]
async fn get_many(
	State(svc): State<RecordService>,
	Query(req): Query<FetchRecordsRequest>,
) -> Result<FetchRecordsResponse, ProblemDetails>
{
	let res = svc.fetch_records(req).await?;

	if res.records.is_empty() {
		Err(Error::NoData)?;
	}

	Ok(res)
}

/// Request payload for `POST /records`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "SubmitRecordRequest")]
pub struct SubmitRecordRequestPayload
{
	/// The ID of the course this record was set on.
	pub course_id: CourseID,

	/// The mode this record was performed in.
	pub mode: Mode,

	/// The styles this record was performed with.
	pub styles: Styles,

	/// The amount of teleports used during this record.
	pub teleports: u32,

	/// The time in seconds.
	pub time: Seconds,

	/// The ID of the player who performed this record.
	pub player_id: SteamID,

	/// Bhop stats for this record.
	pub bhop_stats: BhopStats,
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/records", tag = "Records", security(("CS2 Server" = [])))]
async fn submit_record(
	server: Jwt<jwt::ServerInfo>,
	State(svc): State<RecordService>,
	Json(SubmitRecordRequestPayload {
		course_id,
		mode,
		styles,
		teleports,
		time,
		player_id,
		bhop_stats,
	}): Json<SubmitRecordRequestPayload>,
) -> Result<SubmitRecordResponse, ProblemDetails>
{
	let req = SubmitRecordRequest {
		course_id,
		mode,
		styles,
		teleports,
		time,
		player_id,
		server_id: server.id(),
		bhop_stats,
		plugin_version_id: server.plugin_version_id(),
	};

	let res = svc.submit_record(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/records/{record_id}", tag = "Records", params(
  ("record_id" = RecordID, Path, description = "a record ID"),
))]
async fn get_single(
	State(svc): State<RecordService>,
	Path(record_id): Path<RecordID>,
) -> Result<FetchRecordResponse, ProblemDetails>
{
	let req = FetchRecordRequest { record_id };
	let res = svc
		.fetch_record(req)
		.await?
		.ok_or(Error::RecordDoesNotExist)?;

	Ok(res)
}

/// Request payload for `PATCH /records/{record}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UpdateRecordRequest")]
pub struct UpdateRecordRequestPayload
{
	/// The action you want to perform on this record.
	pub action: UpdateRecordAction,
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/records/{record_id}",
  tag = "Records",
  params(("record_id" = RecordID, Path, description = "a record ID")),
  security(("Browser Session" = ["records"])),
)]
async fn update_record(
	session: Session,
	State(svc): State<RecordService>,
	Path(record_id): Path<RecordID>,
	Json(UpdateRecordRequestPayload { action }): Json<UpdateRecordRequestPayload>,
) -> Result<UpdateRecordResponse, ProblemDetails>
{
	let req = UpdateRecordRequest { record_id, action };
	let res = svc.update_record(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/records/{record_id}/replay", tag = "Records", params(
  ("record_id" = RecordID, Path, description = "a record ID"),
))]
async fn get_replay(
	State(svc): State<RecordService>,
	Path(record_id): Path<RecordID>,
) -> Result<FetchReplayResponse, ProblemDetails>
{
	let req = FetchReplayRequest { record_id };
	let res = svc.fetch_replay(req).await?;

	Ok(res)
}
