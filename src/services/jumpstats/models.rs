//! Request / Response types for this service.

use axum::response::{AppendHeaders, IntoResponse, Response};
use chrono::{DateTime, Utc};
use cs2kz::{JumpType, Mode, SteamID};
use serde::{Deserialize, Serialize};

use crate::num::ClampedU64;
use crate::services::players::PlayerInfo;
use crate::services::plugin::PluginVersionID;
use crate::services::servers::{ServerID, ServerInfo};
use crate::time::Seconds;
use crate::util::{PlayerIdentifier, ServerIdentifier};

crate::macros::make_id! {
	/// An ID uniquely identifying an jumpstat.
	JumpstatID as u64
}

/// Request payload for fetching a jumpstat.
#[derive(Debug)]
pub struct FetchJumpstatRequest
{
	/// The ID of the jumpstat you want to fetch.
	pub jumpstat_id: JumpstatID,
}

/// Response payload for fetching a jumpstat.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchJumpstatResponse
{
	/// The jumpstat's ID.
	pub id: JumpstatID,

	/// The jump type.
	#[serde(rename = "type")]
	pub jump_type: JumpType,

	/// The mode the jump was performed in.
	pub mode: Mode,

	/// The player who performed the jump.
	#[sqlx(flatten)]
	pub player: PlayerInfo,

	/// The server the jump was performed on.
	#[sqlx(flatten)]
	pub server: ServerInfo,

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

	/// When this jumpstat was submitted.
	pub created_on: DateTime<Utc>,
}

impl IntoResponse for FetchJumpstatResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching jumpstats.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct FetchJumpstatsRequest
{
	/// Filter by jump type.
	#[serde(rename = "type")]
	pub jump_type: Option<JumpType>,

	/// Filter by mode.
	pub mode: Option<Mode>,

	/// Filter by required minimum distance.
	pub minimum_distance: Option<f32>,

	/// Filter by player.
	pub player: Option<PlayerIdentifier>,

	/// Filter by server.
	pub server: Option<ServerIdentifier>,

	/// Only include jumpstats submitted after this date.
	pub created_after: Option<DateTime<Utc>>,

	/// Only include jumpstats submitted before this date.
	pub created_before: Option<DateTime<Utc>>,

	/// Maximum number of results to return.
	#[serde(default)]
	#[param(value_type = u64, default = 100, maximum = 1000)]
	pub limit: ClampedU64<100, 1000>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching jumpstats.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchJumpstatsResponse
{
	/// The jumpstats.
	pub jumpstats: Vec<FetchJumpstatResponse>,

	/// How many jumpstats **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchJumpstatsResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for submitting a new jumpstat.
#[derive(Debug)]
pub struct SubmitJumpstatRequest
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

	/// The ID of the server the jump was performed on.
	pub server_id: ServerID,

	/// The ID of the CS2KZ version the server this jump was performed on is
	/// running.
	pub server_plugin_version_id: PluginVersionID,
}

/// Response payload for submitting a new jumpstat.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct SubmitJumpstatResponse
{
	/// The ID of the submitted jumpstat.
	pub jumpstat_id: JumpstatID,
}

impl IntoResponse for SubmitJumpstatResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/jumpstats/{}", self.jumpstat_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}
