use std::net::{Ipv4Addr, SocketAddrV4};

use chrono::{DateTime, Duration, Utc};
use cs2kz::SteamID;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, MySql, Row};
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
	pub reason: BanReason,

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

/// All the reasons players can get banned for.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BanReason {
	AutoBhop,
	AutoStrafe,
}

impl BanReason {
	/// Returns the ban duration for this reason.
	pub fn duration(&self, previous_offenses: u8) -> Duration {
		match (self, previous_offenses) {
			(Self::AutoBhop | Self::AutoStrafe, 0) => Duration::weeks(1),
			(Self::AutoBhop | Self::AutoStrafe, 1) => Duration::weeks(4),
			(Self::AutoBhop | Self::AutoStrafe, 2) => Duration::weeks(12),
			(Self::AutoBhop | Self::AutoStrafe, _) => Duration::weeks(24),
		}
	}
}

impl sqlx::Type<MySql> for BanReason {
	fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
		<&'static str as sqlx::Type<MySql>>::type_info()
	}
}

impl<'query, DB: sqlx::Database> sqlx::Encode<'query, DB> for BanReason
where
	&'static str: sqlx::Encode<'query, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'query>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull {
		<&'static str as sqlx::Encode<'query, DB>>::encode(
			match self {
				BanReason::AutoBhop => "auto_bhop",
				BanReason::AutoStrafe => "auto_strafe",
			},
			buf,
		)
	}
}

impl<'row, DB: sqlx::Database> sqlx::Decode<'row, DB> for BanReason
where
	String: sqlx::Decode<'row, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'row>>::ValueRef,
	) -> std::result::Result<Self, sqlx::error::BoxDynError> {
		#[derive(Debug, thiserror::Error)]
		#[error("unknown variant `{0}`")]
		struct UnknownVariant(String);

		let value = <String as sqlx::Decode<'row, DB>>::decode(value)?;

		match value.as_str() {
			"auto_bhop" => Ok(Self::AutoBhop),
			"auto_strafe" => Ok(Self::AutoStrafe),
			_ => Err(Box::new(UnknownVariant(value))),
		}
	}
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
		let id = crate::sqlx::non_zero!("id" as u32, row)?;
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

		let server = if let Ok(server_id) = crate::sqlx::non_zero!("server_id" as u16, row) {
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
			Some(Player { steam_id, name: row.try_get("banned_by_name")? })
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
		let id = crate::sqlx::non_zero!("unban_id" as u32, row)?;
		let reason = row.try_get("unban_reason")?;
		let created_on = row.try_get("unban_created_on")?;

		let unbanned_by = if let Ok(steam_id) = row.try_get("unbanned_by_steam_id") {
			Some(Player { steam_id, name: row.try_get("unbanned_by_name")? })
		} else {
			None
		};

		Ok(Self { id, reason, unbanned_by, created_on })
	}
}

/// Request body for a new player ban.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewBan {
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,

	/// The reason for the ban.
	pub reason: BanReason,
}

/// Response body for a newly created player ban.
///
/// See [`NewBan`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedBan {
	/// The ban's ID.
	pub ban_id: u32,

	/// When this ban will expire.
	pub expires_on: DateTime<Utc>,
}

/// Request body for updates to a ban.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BanUpdate {
	/// A new ban reason.
	pub reason: Option<BanReason>,

	/// A new expiration date.
	pub expires_on: Option<DateTime<Utc>>,
}

/// Request body for reverting a ban.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewUnban {
	/// The reason for the unban.
	pub reason: String,
}

/// Response body for a reverted ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedUnban {
	/// The ban that was reverted by this unban.
	pub ban_id: u32,

	/// The unban's ID.
	pub unban_id: u32,
}
