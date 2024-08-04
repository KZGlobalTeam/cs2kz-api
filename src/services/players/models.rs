//! Request / Response types for this service.

use std::collections::BTreeMap;

use axum::response::{AppendHeaders, IntoResponse, Response};
use cs2kz::{Mode, SteamID};
use serde::{Deserialize, Deserializer, Serialize};

use crate::net::IpAddr;
use crate::num::ClampedU64;
use crate::services::maps::CourseID;
use crate::services::servers::ServerID;
use crate::services::steam;
use crate::stats::BhopStats;
use crate::time::Seconds;
use crate::util::PlayerIdentifier;

crate::macros::make_id! {
	/// An ID uniquely identifying an in-game session.
	SessionID as u64
}

crate::macros::make_id! {
	/// An ID uniquely identifying an in-game per-course session.
	CourseSessionID as u64
}

/// Basic information about a player.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PlayerInfo
{
	/// The player's name.
	#[sqlx(rename = "player_name")]
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "player_id")]
	pub steam_id: SteamID,
}

/// Request payload for fetching a player.
#[derive(Debug)]
pub struct FetchPlayerRequest
{
	/// An identifier specifying which player you want to fetch.
	pub identifier: PlayerIdentifier,
}

/// Response payload for fetching a player.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchPlayerResponse
{
	/// The player's name and SteamID.
	#[serde(flatten)]
	#[sqlx(flatten)]
	pub info: PlayerInfo,

	/// Whether the player is currently banned.
	pub is_banned: bool,

	/// The player's IP address.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ip_address: Option<IpAddr>,
}

impl IntoResponse for FetchPlayerResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching potentially many players.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct FetchPlayersRequest
{
	/// The maximum amount of players to return.
	#[serde(default)]
	#[param(value_type = u64, default = 100, maximum = 500)]
	pub limit: ClampedU64<100, 500>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching potentially many players.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchPlayersResponse
{
	/// The player data for this request.
	pub players: Vec<FetchPlayerResponse>,

	/// How many players **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchPlayersResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching a player's in-game preferences.
#[derive(Debug)]
pub struct FetchPlayerPreferencesRequest
{
	/// An identifier specifying whose preferences you want to fetch.
	pub identifier: PlayerIdentifier,
}

/// Response payload for fetching a player's in-game preferences.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchPlayerPreferencesResponse
{
	/// The player's preferences.
	pub preferences: serde_json::Value,
}

impl IntoResponse for FetchPlayerPreferencesResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Response payload for fetching a player's Steam profile.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[serde(transparent)]
#[response(status = OK)]
pub struct FetchSteamProfileResponse(#[to_schema] pub steam::User);

impl IntoResponse for FetchSteamProfileResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for registering a new player.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterPlayerRequest
{
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	pub ip_address: IpAddr,
}

/// Response payload for registering a new player.
#[derive(Debug, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct RegisterPlayerResponse
{
	/// The SteamID of the registered player.
	pub player_id: SteamID,
}

impl IntoResponse for RegisterPlayerResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/players/{}", self.player_id.as_u64());
		let headers = AppendHeaders([(http::header::LOCATION, location)]);

		(status, headers).into_response()
	}
}

/// Request payload for updating an existing player.
#[derive(Debug)]
pub struct UpdatePlayerRequest
{
	/// The SteamID of the player you wish to update.
	pub player_id: SteamID,

	/// The ID of the server which sent this request.
	pub server_id: ServerID,

	/// The player's current name.
	pub name: String,

	/// The player's current IP address.
	pub ip_address: IpAddr,

	/// The player's current in-game preferences.
	pub preferences: serde_json::Value,

	/// The player's in-game session.
	pub session: Session,
}

/// An in-game player session.
///
/// A session begins when the player joins the server, and ends when they
/// disconnect. A map change is also considered a disconnect.
///
/// These sessions are used to track various statistics long-term.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct Session
{
	/// How many seconds the player was actively playing.
	pub seconds_active: Seconds,

	/// How many seconds the player spent as a spectator.
	pub seconds_spectating: Seconds,

	/// How many seconds the player was inactive for.
	pub seconds_afk: Seconds,

	/// Bhop stats that span the entire session.
	#[serde(deserialize_with = "BhopStats::deserialize_checked")]
	pub bhop_stats: BhopStats,

	/// Session information per course.
	#[serde(default)]
	pub course_sessions: BTreeMap<CourseID, CourseSession>,
}

/// An in-game session on a specific course in a specific mode.
///
/// This contains data for both VNL and CKZ, which you can iterate over:
///
/// ```no_run
/// use cs2kz_api::services::players::CourseSession;
///
/// let session: CourseSession = todo!();
///
/// for (mode, data) in &session {
///     // ...
/// }
/// ```
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CourseSession
{
	/// The data for [`Mode::Vanilla`].
	#[serde(default, deserialize_with = "CourseSessionData::deserialize_opt_checked")]
	vanilla: Option<CourseSessionData>,

	/// The data for [`Mode::Classic`].
	#[serde(default, deserialize_with = "CourseSessionData::deserialize_opt_checked")]
	classic: Option<CourseSessionData>,
}

impl<'a> IntoIterator for &'a CourseSession
{
	type Item = (Mode, &'a CourseSessionData);
	type IntoIter = CourseSessionIter<'a>;

	fn into_iter(self) -> Self::IntoIter
	{
		CourseSessionIter { vanilla: self.vanilla.as_ref(), classic: self.classic.as_ref() }
	}
}

/// An iterator over [`CourseSessionData`]s stored in a [`CourseSession`].
pub struct CourseSessionIter<'a>
{
	/// The data for [`Mode::Vanilla`].
	vanilla: Option<&'a CourseSessionData>,

	/// The data for [`Mode::Classic`].
	classic: Option<&'a CourseSessionData>,
}

impl<'a> CourseSessionIter<'a>
{
	/// Returns the data for [`Mode::Vanilla`].
	fn vanilla(&mut self) -> Option<(Mode, &'a CourseSessionData)>
	{
		self.vanilla.take().map(|data| (Mode::Vanilla, data))
	}

	/// Returns the data for [`Mode::Classic`].
	fn classic(&mut self) -> Option<(Mode, &'a CourseSessionData)>
	{
		self.classic.take().map(|data| (Mode::Classic, data))
	}
}

impl<'a> Iterator for CourseSessionIter<'a>
{
	type Item = (Mode, &'a CourseSessionData);

	fn next(&mut self) -> Option<Self::Item>
	{
		self.vanilla().or_else(|| self.classic())
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		(0, Some(2))
	}
}

/// The raw data for an in-game session on a specific course.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CourseSessionData
{
	/// How many seconds the player spent with a running timer.
	pub playtime: Seconds,

	/// How many times the player left the start zone of this course.
	pub started_runs: u16,

	/// How many times the player entered the end zone of this course.
	pub finished_runs: u16,

	/// Bhop stats that span just this course.
	#[serde(deserialize_with = "BhopStats::deserialize_checked")]
	pub bhop_stats: BhopStats,
}

impl CourseSessionData
{
	/// Deserializes [`CourseSessionData`] and makes sure the contained data is
	/// logically correct (e.g. `finished_runs <= started_runs`).
	fn deserialize_opt_checked<'de, D>(deserializer: D) -> Result<Option<Self>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let Some(session) = Option::<Self>::deserialize(deserializer)? else {
			return Ok(None);
		};

		if session.started_runs > session.finished_runs {
			return Err(serde::de::Error::custom(
				"`started_runs` cannot be greater than `finished_runs`",
			));
		}

		Ok(Some(session))
	}
}

/// Response payload for updating an existing player.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED)]
pub struct UpdatePlayerResponse
{
	/// The ID of the created in-game session.
	pub session_id: SessionID,

	/// The IDs of the created course sessions.
	pub course_session_ids: Vec<CourseSessionID>,
}

impl IntoResponse for UpdatePlayerResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let body = crate::http::extract::Json(self);

		(status, body).into_response()
	}
}
