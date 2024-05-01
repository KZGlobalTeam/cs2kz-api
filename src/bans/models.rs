//! Types used for describing bans and related concepts.

use std::net::Ipv4Addr;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{database, FromRow, MySql, Row};
use thiserror::Error;
use time::Duration;
use utoipa::ToSchema;

use crate::players::Player;
use crate::servers::ServerInfo;

/// A player ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct Ban {
	/// The ban's ID.
	pub id: u64,

	/// The player affected by this ban.
	pub player: Player,

	/// The server that the ban happened on.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub server: Option<ServerInfo>,

	/// The reason for this ban.
	pub reason: BanReason,

	/// The admin who issued this ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<Player>,

	/// When this ban was submitted.
	pub created_on: DateTime<Utc>,

	/// When this ban will expire.
	pub expires_on: Option<DateTime<Utc>>,

	/// The unban associated with this ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub unban: Option<Unban>,
}

impl FromRow<'_, MySqlRow> for Ban {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("id")?,
			player: Player::from_row(row)?,
			server: ServerInfo::from_row(row).ok(),
			reason: row.try_get("reason")?,
			admin: row
				.try_get("admin_name")
				.and_then(|name| Ok((name, row.try_get("admin_id")?)))
				.map(|(name, steam_id)| Player { name, steam_id })
				.ok(),
			created_on: row.try_get("created_on")?,
			expires_on: row.try_get("expires_on")?,
			unban: Unban::from_row(row).ok(),
		})
	}
}

/// The different reasons for which players can be banned.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BanReason {
	/// Perfect strafes
	AutoStrafe,

	/// Perfect bhops
	AutoBhop,
}

impl BanReason {
	/// A string format compatible with the API.
	#[inline]
	pub const fn as_str(&self) -> &'static str {
		match self {
			BanReason::AutoStrafe => "auto_strafe",
			BanReason::AutoBhop => "auto_bhop",
		}
	}

	/// Calculates the ban duration for this particular reason and the amount of previous bans.
	pub const fn duration(&self, previous_offenses: u8) -> Duration {
		match (self, previous_offenses) {
			(Self::AutoStrafe, 0) => Duration::weeks(2),
			(Self::AutoStrafe, 1) => Duration::weeks(12),
			(Self::AutoStrafe, _) => Duration::weeks(24),
			(Self::AutoBhop, 0) => Duration::weeks(2),
			(Self::AutoBhop, 1) => Duration::weeks(12),
			(Self::AutoBhop, _) => Duration::weeks(24),
		}
	}
}

/// Parsing a [`BanReason`] from a string failed.
#[derive(Debug, Error)]
#[error("`{0}` is not a valid ban reason")]
pub struct InvalidBanReason(String);

impl FromStr for BanReason {
	type Err = InvalidBanReason;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"auto_strafe" => Ok(Self::AutoStrafe),
			"auto_bhop" => Ok(Self::AutoBhop),
			invalid => Err(InvalidBanReason(invalid.to_owned())),
		}
	}
}

impl sqlx::Type<MySql> for BanReason {
	fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
		str::type_info()
	}
}

impl<'q> sqlx::Encode<'q, MySql> for BanReason {
	fn encode_by_ref(
		&self,
		buf: &mut <MySql as database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull {
		self.as_str().encode_by_ref(buf)
	}
}

impl<'q> sqlx::Decode<'q, MySql> for BanReason {
	fn decode(
		value: <MySql as database::HasValueRef<'q>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError> {
		Ok(<&'q str>::decode(value).map(|value| value.parse::<Self>())??)
	}
}

/// A reverted ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct Unban {
	/// The unban's ID.
	pub id: u64,

	/// The reason for the unban.
	pub reason: String,

	/// The admin who reverted this ban.
	pub admin: Option<Player>,

	/// When the ban was reverted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Unban {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			id: row.try_get("unban_id")?,
			reason: row.try_get("unban_reason")?,
			admin: row
				.try_get("unban_admin_name")
				.and_then(|name| Ok((name, row.try_get("unban_admin_id")?)))
				.map(|(name, steam_id)| Player { name, steam_id })
				.ok(),
			created_on: row.try_get("unban_created_on")?,
		})
	}
}

/// Request body for submitting new bans.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewBan {
	/// The player's SteamID.
	pub player_id: SteamID,

	/// The player's IP address.
	pub player_ip: Option<Ipv4Addr>,

	/// The ban reason.
	pub reason: BanReason,
}

/// A newly created ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedBan {
	/// The ban's ID.
	pub ban_id: u64,
}

/// Request body for updating bans.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BanUpdate {
	/// A new ban reason.
	pub reason: Option<BanReason>,

	/// A new expiration date.
	///
	/// Not specifying this at all means the expiration date will not be modified.
	/// If this is explicitly `null`, the expiration date will be deleted and the ban counts as
	/// permanent.
	pub expires_on: Option<Option<DateTime<Utc>>>,
}

/// Request body for reverting a ban.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewUnban {
	/// The reason this ban should be reverted.
	pub reason: String,
}

/// A newly reverted ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedUnban {
	/// The unban's ID.
	pub unban_id: u64,
}
