//! Request / Response types for this service.

use axum::response::{AppendHeaders, IntoResponse, Response};
use chrono::{DateTime, Utc};
use cs2kz::{Mode, RankedStatus, SteamID, Styles, Tier};
use serde::{Deserialize, Serialize};

use crate::num::ClampedU64;
use crate::services::maps::{CourseID, MapID};
use crate::services::players::PlayerInfo;
use crate::services::plugin::PluginVersionID;
use crate::services::servers::{ServerID, ServerInfo};
use crate::stats::BhopStats;
use crate::time::Seconds;
use crate::util::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier};

crate::macros::make_id! {
	/// An ID uniquely identifying a record.
	RecordID as u64
}

/// Request payload for fetching a record.
#[derive(Debug)]
pub struct FetchRecordRequest
{
	/// The record's ID.
	pub record_id: RecordID,
}

/// Response payload for fetching a record.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchRecordResponse
{
	/// The record's ID.
	pub id: RecordID,

	/// The mode used when setting this record.
	pub mode: Mode,

	/// The styles used when setting this record.
	pub styles: Styles,

	/// The amount of teleports used when setting this record.
	pub teleports: u32,

	/// The time in seconds.
	pub time: Seconds,

	/// The course this record was performed on.
	#[sqlx(flatten)]
	pub course: CourseInfo,

	/// The player who performed this record.
	#[sqlx(flatten)]
	pub player: PlayerInfo,

	/// The server which this record was performed on.
	#[sqlx(flatten)]
	pub server: ServerInfo,

	/// Bhop stats for this record.
	#[sqlx(flatten)]
	pub bhop_stats: BhopStats,

	/// When this record was submitted.
	pub created_on: DateTime<Utc>,
}

impl IntoResponse for FetchRecordResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Information about a course a record was performed on.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CourseInfo
{
	/// The course's ID.
	#[sqlx(rename = "course_id")]
	pub id: CourseID,

	/// The course's name.
	#[sqlx(rename = "course_name")]
	pub name: String,

	/// The ID of the map the course belongs to.
	#[sqlx(rename = "course_map_id")]
	pub map_id: MapID,

	/// The name of the map the course belongs to.
	#[sqlx(rename = "course_map_name")]
	pub map_name: String,

	/// The tier of the filter this course belongs to.
	#[sqlx(rename = "course_tier")]
	pub tier: Tier,

	/// The ranked status of the filter this course belongs to.
	#[sqlx(rename = "course_ranked_status")]
	pub ranked_status: RankedStatus,
}

/// Request payload for fetching records.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct FetchRecordsRequest
{
	/// Filter by mode.
	pub mode: Option<Mode>,

	/// Filter by styles.
	///
	/// This is not an exact match; results will contain records that
	/// **include** these styles, but may also have more.
	pub styles: Option<Styles>,

	/// Filter by whether records have teleports or not.
	pub has_teleports: Option<bool>,

	/// Filter by course.
	pub course: Option<CourseIdentifier>,

	/// Filter by map.
	pub map: Option<MapIdentifier>,

	/// Filter by player.
	pub player: Option<PlayerIdentifier>,

	/// Filter by server.
	pub server: Option<ServerIdentifier>,

	/// Only include top records.
	///
	/// That is, only include the fastest time per player per filter.
	#[serde(default)]
	pub top: bool,

	/// In which order to sort the results.
	///
	/// This will have different defaults depending on `sort_by`, but if this
	/// field is specified, the order is forced.
	pub sort_order: Option<SortOrder>,

	/// Which property to sort the results after.
	#[serde(default)]
	pub sort_by: SortRecordsBy,

	/// Only include records submitted after this date.
	pub created_after: Option<DateTime<Utc>>,

	/// Only include records submitted before this date.
	pub created_before: Option<DateTime<Utc>>,

	/// The maximum amount of records to return.
	#[serde(default)]
	#[param(value_type = u64, default = 100, maximum = 500)]
	pub limit: ClampedU64<100, 500>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// How to sort results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder
{
	/// Sort results from low to high.
	Ascending,

	/// Sort results from high to low.
	Descending,
}

impl From<SortRecordsBy> for SortOrder
{
	fn from(sort_by: SortRecordsBy) -> Self
	{
		match sort_by {
			SortRecordsBy::Time => Self::Ascending,
			SortRecordsBy::Date => Self::Descending,
		}
	}
}

/// Which property to sort results by.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortRecordsBy
{
	/// Sort results by time.
	Time,

	/// Sort results by creation date.
	#[default]
	Date,
}

/// Response payload for fetching records.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchRecordsResponse
{
	/// The records.
	pub records: Vec<FetchRecordResponse>,

	/// How many records **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchRecordsResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching a record's replay.
#[derive(Debug)]
pub struct FetchReplayRequest
{
	/// The record's ID.
	pub record_id: RecordID,
}

/// Response payload for fetching a record's replay.
#[derive(Debug, utoipa::IntoResponses)]
#[response(status = SERVICE_UNAVAILABLE)]
pub struct FetchReplayResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for FetchReplayResponse
{
	fn into_response(self) -> Response
	{
		// TODO: actually implement a replay system!
		http::StatusCode::SERVICE_UNAVAILABLE.into_response()
	}
}

/// Request payload for submitting a new record.
#[derive(Debug)]
pub struct SubmitRecordRequest
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

	/// The ID of the server this record was performed on.
	pub server_id: ServerID,

	/// Bhop stats for this record.
	pub bhop_stats: BhopStats,

	/// The ID of the CS2KZ version the server this record was performed on is
	/// currently running.
	pub plugin_version_id: PluginVersionID,
}

/// Response payload for submitting a new record.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct SubmitRecordResponse
{
	/// The generated record ID.
	pub record_id: RecordID,
}

impl IntoResponse for SubmitRecordResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/records/{}", self.record_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}

/// Request payload for updating a record.
#[derive(Debug)]
pub struct UpdateRecordRequest
{
	/// The ID of the record you wish to update.
	pub record_id: RecordID,

	/// The action you want to perform on the record.
	pub action: UpdateRecordAction,
}

/// Actions you can perform on a record.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UpdateRecordAction
{
	/// Change the status of the record.
	ChangeStatus
	{
		/// The new status you want to set.
		new_status: RecordStatus,
	},
}

/// The different statuses for records.
///
/// Only "default" records are included when fetching records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus
{
	/// The default status.
	Default,

	/// Mark a record as "suspicious".
	///
	/// This indicates that it might be cheated and will be investigated by an
	/// admin.
	Suspicious,

	/// Mark a record as "cheated".
	Cheated,

	/// The "deleted" status.
	///
	/// Records are never _actually_ deleted, but just moved into a different
	/// table. To "delete" a record, give it this status.
	Wiped,
}

impl RecordStatus
{
	/// Returns the name of the SQL table that corresponds to this status.
	pub(super) fn table_name(&self) -> &'static str
	{
		match self {
			RecordStatus::Default => "Records",
			RecordStatus::Suspicious => "SuspiciousRecords",
			RecordStatus::Cheated => "CheatedRecords",
			RecordStatus::Wiped => "WipedRecords",
		}
	}
}

/// Response payload for updating a record.
#[derive(Debug, Serialize)]
pub struct UpdateRecordResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for UpdateRecordResponse
{
	fn into_response(self) -> Response
	{
		http::StatusCode::OK.into_response()
	}
}

crate::openapi::responses::no_content!(UpdateRecordResponse);
