//! Request / Response types for this service.

use axum::response::{AppendHeaders, IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::num::ClampedU64;

#[doc(hidden)]
pub(crate) mod version;
pub use version::PluginVersion;

crate::macros::make_id! {
	/// A unique identifier for CS2KZ versions.
	PluginVersionID as u16
}

/// An identifier for a CS2KZ plugin version.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PluginVersionIdentifier
{
	/// A uniquely identifying ID.
	ID(PluginVersionID),

	/// A semantic version.
	SemVer(PluginVersion),

	/// A git revision.
	GitRev(String),
}

impl PluginVersionIdentifier
{
	/// Returns the value stored inside the `ID` variant, if available.
	pub fn as_id(&self) -> Option<PluginVersionID>
	{
		if let Self::ID(id) = *self {
			Some(id)
		} else {
			None
		}
	}

	/// Returns the value stored inside the `SemVer` variant, if available.
	pub fn as_semver(&self) -> Option<&PluginVersion>
	{
		if let Self::SemVer(version) = self {
			Some(version)
		} else {
			None
		}
	}

	/// Returns the value stored inside the `GitRev` variant, if available.
	pub fn as_git_rev(&self) -> Option<&str>
	{
		if let Self::GitRev(rev) = self {
			Some(rev.as_str())
		} else {
			None
		}
	}
}

/// Request payload for fetching a plugin version.
#[derive(Debug)]
pub struct FetchPluginVersionRequest
{
	/// Identifier specifying which plugin version you want to fetch.
	pub ident: PluginVersionIdentifier,
}

/// Request payload for fetching a plugin version.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchPluginVersionResponse
{
	/// The ID of this version.
	pub id: PluginVersionID,

	/// The semver representation of this version.
	pub semver: PluginVersion,

	/// The git revision associated with this version.
	pub git_revision: String,

	/// When this version was submitted.
	#[serde(with = "time::serde::rfc3339")]
	pub created_on: OffsetDateTime,
}

impl IntoResponse for FetchPluginVersionResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching plugin versions.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct FetchPluginVersionsRequest
{
	/// The maximum amount of versions to return.
	#[serde(default)]
	#[param(value_type = u64, default = 50, maximum = 1000)]
	pub limit: ClampedU64<50, 1000>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching plugin versions.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchPluginVersionsResponse
{
	/// The versions.
	pub versions: Vec<FetchPluginVersionResponse>,

	/// How many versions **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchPluginVersionsResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for submitting a new plugin version.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubmitPluginVersionRequest
{
	/// The semver representation of this version.
	#[schema(value_type = str)]
	pub semver: PluginVersion,

	/// The git revision associated with this version.
	pub git_revision: String,
}

/// Response payload for submitting a new plugin version.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct SubmitPluginVersionResponse
{
	/// The generated version ID.
	pub plugin_version_id: PluginVersionID,
}

impl IntoResponse for SubmitPluginVersionResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/plugin/versions/{}", self.plugin_version_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}
