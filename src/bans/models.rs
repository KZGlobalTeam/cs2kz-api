use std::net::{Ipv4Addr, SocketAddrV4};

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::servers::Server;

/// A player ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct Ban {
	/// The ban's ID.
	pub id: u32,

	/// The player.
	pub player: BannedPlayer,

	/// The reason for the ban.
	// TODO(AlphaKeks): make this an enum?
	pub reason: String,

	/// The server the player was banned on (if any).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub server: Option<Server>,

	/// The cs2kz plugin version at the time of the ban.
	///
	/// This is either the version the [`server`] was currently running on, or the latest
	/// current version, if they player got banned by an admin directly.
	///
	/// [`server`]: Ban::server
	#[schema(value_type = String)]
	pub plugin_version: Version,

	/// The admin who issued this ban (if any).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub banned_by: Option<Player>,

	/// When this ban was issued.
	pub created_on: DateTime<Utc>,

	/// When this ban will expire.
	pub expires_on: Option<DateTime<Utc>>,

	/// The corresponding unban to this ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unban: Option<Unban>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BannedPlayer {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's name.
	pub name: String,

	/// The player's IP address at the time of their ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,
}

impl FromRow<'_, MySqlRow> for Ban {
	fn from_row(row: &'_ MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;
		let player = BannedPlayer {
			steam_id: row.try_get("player_id")?,
			name: row.try_get("player_name")?,
			ip_address: row
				.try_get::<&str, _>("player_ip")?
				.parse()
				.map(Some)
				.map_err(|err| sqlx::Error::ColumnDecode {
					index: String::from("player_ip"),
					source: Box::new(err),
				})?,
		};

		let reason = row.try_get("reason")?;

		let server = if let Ok(server_id) = row.try_get("server_id") {
			Some(Server {
				id: server_id,
				name: row.try_get("server_name")?,
				ip_address: {
					let ip = row
						.try_get::<&str, _>("server_ip_address")?
						.parse()
						.map_err(|err| sqlx::Error::ColumnDecode {
							index: String::from("server_ip_address"),
							source: Box::new(err),
						})?;

					let port = row.try_get("server_port")?;

					SocketAddrV4::new(ip, port)
				},
				owned_by: Player {
					steam_id: row.try_get("server_owner_steam_id")?,
					name: row.try_get("server_owner_name")?,
					is_banned: row.try_get("server_owner_is_banned")?,
				},
				approved_on: row.try_get("server_approved_on")?,
			})
		} else {
			None
		};

		let plugin_version = row
			.try_get::<&str, _>("plugin_version")?
			.parse()
			.map_err(|err| sqlx::Error::ColumnDecode {
				index: String::from("plugin_version"),
				source: Box::new(err),
			})?;

		let banned_by = if let Ok(steam_id) = row.try_get("banned_by_steam_id") {
			Some(Player {
				steam_id,
				name: row.try_get("banned_by_name")?,
				is_banned: row.try_get("banned_by_is_banned")?,
			})
		} else {
			None
		};

		let created_on = row.try_get("created_on")?;
		let expires_on = row.try_get("expires_on")?;
		let unban = Unban::from_row(row).ok();

		Ok(Self {
			id,
			player,
			reason,
			server,
			plugin_version,
			banned_by,
			created_on,
			expires_on,
			unban,
		})
	}
}

/// A reverted ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct Unban {
	/// The ID of this unban.
	pub id: u32,

	/// The reason for the unban.
	pub reason: String,

	/// The player who reverted this ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unbanned_by: Option<Player>,

	/// When this unban was created.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Unban {
	fn from_row(row: &'_ MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("unban_id")?;
		let reason = row.try_get("unban_reason")?;
		let created_on = row.try_get("unban_created_on")?;

		let unbanned_by = if let Ok(steam_id) = row.try_get("unbanned_by_steam_id") {
			Some(Player {
				steam_id,
				name: row.try_get("unbanned_by_name")?,
				is_banned: row.try_get("unbanned_by_is_banned")?,
			})
		} else {
			None
		};

		Ok(Self { id, reason, unbanned_by, created_on })
	}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewBan {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,

	/// The reason for the ban.
	pub reason: String,
}

/// A newly created [`Ban`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedBan {
	/// The ban's ID.
	pub ban_id: u32,
}

/// An update to a [`Ban`].
#[derive(Debug, Deserialize, ToSchema)]
pub struct BanUpdate {
	/// A new ban reason.
	pub reason: Option<String>,

	/// A new expiration date.
	pub expires_on: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewUnban {
	/// The reason for the unban.
	pub reason: String,
}

/// A newly created Unban.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedUnban {
	/// The unban's ID.
	pub unban_id: u32,
}
