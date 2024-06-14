//! Types for modeling KZ player bans.

use std::net::IpAddr;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{database, FromRow, MySql, Row};
use thiserror::Error;
use time::Duration;
use utoipa::ToSchema;

use crate::make_id;
use crate::players::Player;
use crate::servers::ServerInfo;

make_id!(BanID as u64);
make_id!(UnbanID as u64);

/// A player ban.
#[derive(Debug, Serialize, ToSchema)]
pub struct Ban {
	/// The ban's ID.
	pub id: BanID,

	/// The player who the ban applies to.
	pub player: Player,

	/// The server the player was banned on.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub server: Option<ServerInfo>,

	/// The reason the player was banned for.
	pub reason: BanReason,

	/// The admin who banned the player.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<Player>,

	/// When this ban was submitted.
	pub created_on: DateTime<Utc>,

	/// When this ban will expire.
	pub expires_on: Option<DateTime<Utc>>,

	/// The corresponding unban to this ban (if any).
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

/// Ban reasons.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum BanReason {
	AutoStrafe,
	AutoBhop,
}

impl BanReason {
	/// Stringified version that is also expected when parsing a string into a [`BanReason`].
	pub const fn as_str(&self) -> &'static str {
		match self {
			BanReason::AutoStrafe => "auto_strafe",
			BanReason::AutoBhop => "auto_bhop",
		}
	}

	/// Calculates the ban duration given the amount of previous bans.
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

/// An error for parsing ban reasons.
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
		<str as sqlx::Type<MySql>>::type_info()
	}
}

impl<'q> sqlx::Encode<'q, MySql> for BanReason {
	fn encode_by_ref(
		&self,
		buf: &mut <MySql as database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull {
		<&'q str as sqlx::Encode<'q, MySql>>::encode_by_ref(&self.as_str(), buf)
	}
}

impl<'q> sqlx::Decode<'q, MySql> for BanReason {
	fn decode(
		value: <MySql as database::HasValueRef<'q>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError> {
		Ok(<&'q str as sqlx::Decode<'q, MySql>>::decode(value)
			.map(|value| value.parse::<Self>())??)
	}
}

/// Reversion of a `Ban`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Unban {
	/// The unban's ID.
	pub id: UnbanID,

	/// The reason for the unban.
	pub reason: String,

	/// The admin who reverted the ban.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<Player>,

	/// When this ban was reverted.
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

/// Request payload for submitting a new ban.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct NewBan {
	/// The SteamID of the player who should be banned.
	pub player_id: SteamID,

	/// The IP address of the player who should be banned.
	#[schema(value_type = Option<String>)]
	pub player_ip: Option<IpAddr>,

	/// The reason for the ban.
	pub reason: BanReason,
}

/// Response body for submitting a new ban.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedBan {
	/// The ban's ID.
	pub ban_id: BanID,
}

/// Request payload for updating an existing ban.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct BanUpdate {
	/// A new ban reason.
	pub reason: Option<BanReason>,

	/// A new expiration date.
	///
	/// If this field is omitted, nothing will happen.
	/// If it is explicitly set to `null`, the expiration date will be set to `NULL`
	/// (permanent).
	pub expires_on: Option<Option<DateTime<Utc>>>,
}

/// Request payload for submitting an unban.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewUnban {
	/// The reason for the unban.
	pub reason: String,
}

/// Response body for creating a new unban.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedUnban {
	/// The unban's ID.
	pub unban_id: UnbanID,
}
