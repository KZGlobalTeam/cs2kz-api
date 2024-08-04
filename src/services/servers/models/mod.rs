//! A service for managing KZ servers.

use axum::response::{AppendHeaders, IntoResponse, Response};
use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};

use crate::num::ClampedU64;
use crate::services::plugin::PluginVersion;
use crate::util::{PlayerIdentifier, ServerIdentifier};

#[doc(hidden)]
pub(crate) mod host;
pub use host::Host;

#[doc(hidden)]
pub(crate) mod api_key;
pub use api_key::ApiKey;

crate::macros::make_id! {
	/// A unique identifier for an approved CS2KZ server.
	ServerID as u16
}

/// Basic information about a server.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ServerInfo
{
	/// The server's ID.
	#[sqlx(rename = "server_id")]
	pub id: ServerID,

	/// The server's name.
	#[sqlx(rename = "server_name")]
	pub name: String,
}

/// Request payload for fetching information about a server.
#[derive(Debug)]
pub struct FetchServerRequest
{
	/// An identifier specifying which server you want to fetch information
	/// about.
	pub identifier: ServerIdentifier,
}

/// Response payload for fetching information about a server.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchServerResponse
{
	/// The server's ID.
	pub id: ServerID,

	/// The server's name.
	pub name: String,

	/// The server's host IP / domain.
	pub host: Host,

	/// The server's port.
	pub port: u16,

	/// The server's owner.
	#[sqlx(flatten)]
	pub owner: ServerOwner,

	/// When this server was approved.
	pub created_on: DateTime<Utc>,
}

impl IntoResponse for FetchServerResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Information about a server owner.
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ServerOwner
{
	/// The owner's name.
	#[sqlx(rename = "owner_name")]
	pub name: String,

	/// The owner's SteamID.
	#[sqlx(rename = "owner_id")]
	pub steam_id: SteamID,
}

/// Request payload for fetching information about servers.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct FetchServersRequest
{
	/// Filter by server name.
	pub name: Option<String>,

	/// Filter by server host.
	pub host: Option<Host>,

	/// Filter by server owner.
	pub owned_by: Option<PlayerIdentifier>,

	/// Filter by approval date.
	pub created_after: Option<DateTime<Utc>>,

	/// Filter by approval date.
	pub created_before: Option<DateTime<Utc>>,

	/// The maximum amount of servers to return.
	#[serde(default)]
	#[param(value_type = u64, default = 50, maximum = 500)]
	pub limit: ClampedU64<50, 500>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching information about servers.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchServersResponse
{
	/// The servers.
	pub servers: Vec<FetchServerResponse>,

	/// How many servers **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchServersResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for registering a new server.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterServerRequest
{
	/// The server's name.
	pub name: String,

	/// The server's host IP / domain.
	pub host: Host,

	/// The server's port.
	pub port: u16,

	/// The server owner's SteamID.
	pub owner_id: SteamID,
}

/// Response payload for registering a new server.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct RegisterServerResponse
{
	/// The server's ID.
	pub server_id: ServerID,

	/// The server's API key.
	pub api_key: ApiKey,
}

impl IntoResponse for RegisterServerResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/servers/{}", self.server_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}

/// Request payload for updating a server.
#[derive(Debug)]
pub struct UpdateServerRequest
{
	/// The server's ID.
	pub server_id: ServerID,

	/// A new name.
	pub new_name: Option<String>,

	/// A new host.
	pub new_host: Option<Host>,

	/// A new port.
	pub new_port: Option<u16>,

	/// SteamID of a new owner.
	pub new_owner: Option<SteamID>,
}

impl UpdateServerRequest
{
	/// Checks if this update does not contain any changes.
	pub fn is_empty(&self) -> bool
	{
		let Self { server_id: _, new_name, new_host, new_port, new_owner } = self;

		new_name.is_none() && new_host.is_none() && new_port.is_none() && new_owner.is_none()
	}
}

/// Response payload for updating a server.
#[derive(Debug)]
pub struct UpdateServerResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for UpdateServerResponse
{
	fn into_response(self) -> Response
	{
		http::StatusCode::NO_CONTENT.into_response()
	}
}

crate::openapi::responses::no_content!(UpdateServerResponse);

/// Request payload for resetting a server's API key.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ResetKeyRequest
{
	/// The server's ID.
	pub server_id: ServerID,
}

/// Response payload for resetting a server's API key.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED)]
pub struct ResetKeyResponse
{
	/// The generated key.
	pub key: ApiKey,
}

impl IntoResponse for ResetKeyResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let body = crate::http::extract::Json(self);

		(status, body).into_response()
	}
}

/// Request payload for deleting a server's API key.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DeleteKeyRequest
{
	/// The server's ID.
	pub server_id: ServerID,
}

/// Response payload for deleting a server's API key.
#[derive(Debug, Serialize)]
pub struct DeleteKeyResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for DeleteKeyResponse
{
	fn into_response(self) -> Response
	{
		http::StatusCode::NO_CONTENT.into_response()
	}
}

crate::openapi::responses::no_content!(DeleteKeyResponse);

/// Request payload for generating a temporary access token.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GenerateAccessTokenRequest
{
	/// The server's API key.
	pub key: ApiKey,

	/// The CS2KZ version the server is currently running.
	pub plugin_version: PluginVersion,
}

/// Response payload for generating a temporary access token.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED)]
pub struct GenerateAccessTokenResponse
{
	/// The token.
	pub token: String,
}

impl IntoResponse for GenerateAccessTokenResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let body = crate::http::extract::Json(self);

		(status, body).into_response()
	}
}
