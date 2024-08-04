//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::{JumpType, Mode, SteamID};
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	Error,
	FetchJumpstatResponse,
	FetchJumpstatsRequest,
	FetchJumpstatsResponse,
	JumpstatID,
	JumpstatService,
	SubmitJumpstatRequest,
	SubmitJumpstatResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::jwt::{self, JwtLayer};
use crate::services::auth::Jwt;
use crate::services::jumpstats::FetchJumpstatRequest;
use crate::time::Seconds;

impl From<JumpstatService> for Router
{
	fn from(svc: JumpstatService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(JwtLayer::<jwt::ServerInfo>::new(svc.auth_svc.clone()));

		let no_cors = Router::new()
			.route("/", routing::post(submit).route_layer(auth))
			.with_state(svc.clone());

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:id", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		no_cors.merge(public)
	}
}

/// Fetch many jumpstats.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/jumpstats", tag = "Jumpstats", params(FetchJumpstatsRequest))]
async fn get_many(
	State(svc): State<JumpstatService>,
	Query(req): Query<FetchJumpstatsRequest>,
) -> Result<FetchJumpstatsResponse, ProblemDetails>
{
	let res = svc.fetch_jumpstats(req).await?;

	if res.jumpstats.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(res)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "SubmitJumpstatRequest")]
#[doc(hidden)]
pub(crate) struct SubmitJumpstatRequestPayload
{
	/// The jump type.
	pub jump_type: JumpType,

	/// The mode the jump was performed in.
	pub mode: Mode,

	/// The SteamID of the player who performed the jump.
	pub player_id: SteamID,

	/// How many strafes the player performed during the jump.
	pub strafes: u8,

	/// The distance cleared by the jump.
	pub distance: f32,

	/// The % of airtime spent gaining speed.
	pub sync: f32,

	/// The speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The amount of time spent pressing both strafe keys.
	pub overlap: Seconds,

	/// The amount of time spent pressing keys but not gaining speed.
	pub bad_angles: Seconds,

	/// The amount of time spent doing nothing.
	pub dead_air: Seconds,

	/// The maximum height reached during the jump.
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing position deviates from the jumpoff position.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// The amount of time spent mid-air.
	pub airtime: Seconds,
}

#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/jumpstats", tag = "Jumpstats", security(("CS2 Server" = [])))]
async fn submit(
	server: Jwt<jwt::ServerInfo>,
	State(svc): State<JumpstatService>,
	Json(SubmitJumpstatRequestPayload {
		jump_type,
		mode,
		player_id,
		strafes,
		distance,
		sync,
		pre,
		max,
		overlap,
		bad_angles,
		dead_air,
		height,
		airpath,
		deviation,
		average_width,
		airtime,
	}): Json<SubmitJumpstatRequestPayload>,
) -> Result<SubmitJumpstatResponse, ProblemDetails>
{
	let req = SubmitJumpstatRequest {
		jump_type,
		mode,
		player_id,
		strafes,
		distance,
		sync,
		pre,
		max,
		overlap,
		bad_angles,
		dead_air,
		height,
		airpath,
		deviation,
		average_width,
		airtime,
		server_id: server.id(),
		server_plugin_version_id: server.plugin_version_id(),
	};

	let res = svc.submit_jumpstat(req).await?;

	Ok(res)
}

/// Fetch a jumpstat by its ID.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/jumpstats/{jumpstat_id}", tag = "Jumpstats", params(
  ("jumpstat_id" = JumpstatID, Path, description = "a jumpstat's ID"),
))]
async fn get_single(
	State(svc): State<JumpstatService>,
	Path(jumpstat_id): Path<JumpstatID>,
) -> Result<FetchJumpstatResponse, ProblemDetails>
{
	let req = FetchJumpstatRequest { jumpstat_id };
	let res = svc
		.fetch_jumpstat(req)
		.await?
		.ok_or(Error::JumpstatDoesNotExist { jumpstat_id })?;

	Ok(res)
}
