//! Request / Response types for this service.

use axum::response::{IntoResponse, Response};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};

use crate::num::ClampedU64;
use crate::services::auth::session::user::Permissions;

/// Request payload for fetching an admin.
#[derive(Debug)]
pub struct FetchAdminRequest
{
	/// The admin's SteamID.
	pub user_id: SteamID,
}

/// Response payload for fetching an admin.
#[derive(Debug, Serialize, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchAdminResponse
{
	/// The admin's name.
	pub name: String,

	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's permissions.
	pub permissions: Permissions,
}

impl IntoResponse for FetchAdminResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for fetching many admins.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct FetchAdminsRequest
{
	/// Only include admins with these permissions.
	#[serde(default)]
	pub required_permissions: Permissions,

	/// The maximum amount of admins to return.
	#[serde(default)]
	#[param(value_type = u64)]
	pub limit: ClampedU64<{ u64::MAX }>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching many admins.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchAdminsResponse
{
	/// The admins.
	pub admins: Vec<FetchAdminResponse>,

	/// How many admins **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchAdminsResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for updating a user's permissions.
#[derive(Debug)]
pub struct SetPermissionsRequest
{
	/// The user's SteamID.
	pub user_id: SteamID,

	/// The permissions to set for the user.
	pub permissions: Permissions,
}

/// Response payload for updating a user's permissions.
#[derive(Debug)]
pub struct SetPermissionsResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for SetPermissionsResponse
{
	fn into_response(self) -> Response
	{
		http::StatusCode::NO_CONTENT.into_response()
	}
}

crate::openapi::responses::no_content!(SetPermissionsResponse);
