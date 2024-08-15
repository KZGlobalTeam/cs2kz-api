//! Request / Response types for this service.

use axum::response::{AppendHeaders, IntoResponse, Response};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::net::IpAddr;
use crate::num::ClampedU64;
use crate::services::players::PlayerInfo;
use crate::services::plugin::PluginVersionID;
use crate::services::servers::{ServerID, ServerInfo};
use crate::util::{PlayerIdentifier, ServerIdentifier};

#[doc(hidden)]
pub(crate) mod ban_reason;
pub use ban_reason::BanReason;

mod unban_reason;
pub use unban_reason::UnbanReason;

crate::macros::make_id! {
	/// An ID uniquely identifying a ban.
	BanID as u64
}

crate::macros::make_id! {
	/// An ID uniquely identifying an unban.
	UnbanID as u64
}

/// Request payload for fetching a ban.
#[derive(Debug)]
pub struct FetchBanRequest
{
	/// The ID of the ban you want to fetch.
	pub ban_id: BanID,
}

/// Response payload for fetching a ban.
#[derive(Debug, Serialize, utoipa::ToSchema, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchBanResponse
{
	/// The ban's ID.
	pub id: BanID,

	/// The player who was banned.
	pub player: PlayerInfo,

	/// The server the player was banned on.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub server: Option<ServerInfo>,

	/// The admin the player was banned by.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<PlayerInfo>,

	/// The reason the player was banned.
	pub reason: BanReason,

	/// When this ban was created.
	#[serde(with = "time::serde::rfc3339")]
	pub created_on: OffsetDateTime,

	/// When this ban will expire.
	///
	/// This is `null` for permanent bans.
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub expires_on: Option<OffsetDateTime>,

	/// The corresponding unban for this ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unban: Option<Unban>,
}

// We can't derive this because `#[sqlx(flatten)]` does not support `Option<T>`.
impl<'r, R> sqlx::FromRow<'r, R> for FetchBanResponse
where
	R: sqlx::Row,
	for<'a> &'a str: sqlx::ColumnIndex<R>,
	BanID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	UnbanID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	ServerID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	SteamID: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	String: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	BanReason: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	UnbanReason: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
	OffsetDateTime: sqlx::Type<R::Database> + sqlx::Decode<'r, R::Database>,
{
	fn from_row(row: &'r R) -> sqlx::Result<Self>
	{
		let id = row.try_get("id")?;
		let player = PlayerInfo::from_row(row)?;
		let server = {
			let id: Option<ServerID> = row.try_get("server_id")?;
			let name: Option<String> = row.try_get("server_name")?;

			Option::zip(id, name).map(|(id, name)| ServerInfo { id, name })
		};
		let admin = {
			let name: Option<String> = row.try_get("admin_name")?;
			let steam_id: Option<SteamID> = row.try_get("admin_id")?;

			Option::zip(name, steam_id).map(|(name, steam_id)| PlayerInfo { name, steam_id })
		};
		let reason = row.try_get("reason")?;
		let created_on = row.try_get("created_on")?;
		let expires_on = row.try_get("expires_on")?;
		let unban = {
			let id: Option<UnbanID> = row.try_get("unban_id")?;
			let reason: Option<UnbanReason> = row.try_get("unban_reason")?;
			let admin = {
				let name: Option<String> = row.try_get("unban_admin_name")?;
				let steam_id: Option<SteamID> = row.try_get("unban_admin_id")?;

				Option::zip(name, steam_id).map(|(name, steam_id)| PlayerInfo { name, steam_id })
			};
			let created_on = row.try_get("unban_created_on")?;

			Option::zip(id, reason)
				.zip(created_on)
				.map(|((id, reason), created_on)| Unban { id, reason, admin, created_on })
		};

		Ok(Self { id, player, server, admin, reason, created_on, expires_on, unban })
	}
}

impl IntoResponse for FetchBanResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// A reverted ban.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Unban
{
	/// The unban's ID.
	pub id: UnbanID,

	/// The reason for the unban.
	#[schema(value_type = str)]
	pub reason: UnbanReason,

	/// The admin who reverted the ban.
	pub admin: Option<PlayerInfo>,

	/// When the ban was reverted.
	#[serde(with = "time::serde::rfc3339")]
	pub created_on: OffsetDateTime,
}

/// Request payload for fetching bans.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct FetchBansRequest
{
	/// Filter by player.
	pub player: Option<PlayerIdentifier>,

	/// Filter by server.
	pub server: Option<ServerIdentifier>,

	/// Filter by ban reason.
	pub reason: Option<BanReason>,

	/// Only include bans that have (not) already expired / have been reverted.
	pub unbanned: Option<bool>,

	/// Filter by admin who created the ban.
	pub banned_by: Option<PlayerIdentifier>,

	/// Filter by admin who created the unban.
	pub unbanned_by: Option<PlayerIdentifier>,

	/// Filter by creation date.
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub created_after: Option<OffsetDateTime>,

	/// Filter by creation date.
	#[serde(default, with = "time::serde::rfc3339::option")]
	pub created_before: Option<OffsetDateTime>,

	/// The maximum amount of bans to return.
	#[serde(default)]
	#[param(value_type = u64, default = 100, maximum = 500)]
	pub limit: ClampedU64<100, 500>,

	/// Pagination offset.
	#[serde(default)]
	#[param(value_type = u64)]
	pub offset: ClampedU64,
}

/// Response payload for fetching bans.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = OK)]
pub struct FetchBansResponse
{
	/// The bans.
	pub bans: Vec<FetchBanResponse>,

	/// How many bans **could have been** fetched, if there was no limit.
	pub total: u64,
}

impl IntoResponse for FetchBansResponse
{
	fn into_response(self) -> Response
	{
		crate::http::extract::Json(self).into_response()
	}
}

/// Request payload for banning a player.
#[derive(Debug)]
pub struct BanRequest
{
	/// The player's SteamID.
	pub player_id: SteamID,

	/// The player's IP address.
	pub player_ip: Option<IpAddr>,

	/// The reason for the ban.
	pub reason: BanReason,

	/// Who issued this ban?
	pub banned_by: BannedBy,
}

/// Enum indicating who issued a [`BanRequest`].
#[derive(Debug)]
pub enum BannedBy
{
	/// The ban was issued by a server.
	Server
	{
		/// The server's ID.
		id: ServerID,

		/// The ID of the CS2KZ version the server is currently running.
		plugin_version_id: PluginVersionID,
	},

	/// The ban was issued by an admin.
	Admin
	{
		/// The admin's SteamID.
		steam_id: SteamID,
	},
}

/// Response payload for banning a player.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct BanResponse
{
	/// The ID of the ban that was just created.
	pub ban_id: BanID,
}

impl IntoResponse for BanResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/bans/{}", self.ban_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}

/// Request payload for updating a ban.
#[derive(Debug)]
pub struct UpdateBanRequest
{
	/// The ban's ID.
	pub ban_id: BanID,

	/// A new ban reason.
	pub new_reason: Option<String>,

	/// A new expiration date.
	pub new_expiration_date: Option<OffsetDateTime>,
}

impl UpdateBanRequest
{
	/// Checks whether this update contains no changes.
	pub fn is_empty(&self) -> bool
	{
		self.new_reason.is_none() && self.new_expiration_date.is_none()
	}
}

/// Response payload for updating a ban.
#[derive(Debug)]
pub struct UpdateBanResponse
{
	/// non-exhaustive
	pub(super) _priv: (),
}

impl IntoResponse for UpdateBanResponse
{
	fn into_response(self) -> Response
	{
		http::StatusCode::NO_CONTENT.into_response()
	}
}

crate::openapi::responses::no_content!(UpdateBanResponse);

/// Request payload for reverting a ban.
#[derive(Debug)]
pub struct UnbanRequest
{
	/// The ID of the ban to revert.
	pub ban_id: BanID,

	/// The reason for the unban.
	pub reason: UnbanReason,

	/// SteamID of the admin who issued this unban.
	pub admin_id: SteamID,
}

/// Response payload for reverting a ban.
#[derive(Debug, Serialize, utoipa::IntoResponses)]
#[response(status = CREATED, headers(
  ("Location", description = "a relative uri to fetch the created resource"),
))]
pub struct UnbanResponse
{
	/// The ID of the ban this unban relates to.
	#[serde(skip_serializing)]
	pub(super) ban_id: BanID,

	/// The ID of the created unban.
	pub unban_id: UnbanID,
}

impl IntoResponse for UnbanResponse
{
	fn into_response(self) -> Response
	{
		let status = http::StatusCode::CREATED;
		let location = format!("/bans/{}", self.ban_id);
		let headers = AppendHeaders([(http::header::LOCATION, location)]);
		let body = crate::http::extract::Json(self);

		(status, headers, body).into_response()
	}
}
