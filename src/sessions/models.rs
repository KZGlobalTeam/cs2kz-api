use std::net::SocketAddrV4;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::players::Player;
use crate::servers::Server;

/// Response body for player sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Session {
	/// The session's ID.
	pub id: u32,

	/// The player associated with the session.
	pub player: Player,

	/// The server which submitted this session.
	pub server: Server,

	/// Playtime statistics.
	pub time: TimeSpent,

	/// Bhop statistics.
	pub bhop_stats: BhopStats,

	/// When this session was submitted.
	pub created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for Session {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;

		let player =
			Player { steam_id: row.try_get("steam_id")?, name: row.try_get("player_name")? };

		let server = Server {
			id: row.try_get("server_id")?,
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
		};

		let time = TimeSpent {
			active: row.try_get("time_active").map(Duration::from_secs)?,
			spectating: row.try_get("time_spectating").map(Duration::from_secs)?,
			afk: row.try_get("time_afk").map(Duration::from_secs)?,
		};

		let bhop_stats = BhopStats {
			perfs: row.try_get("perfs")?,
			tick0: row.try_get("bhops_tick0")?,
			tick1: row.try_get("bhops_tick1")?,
			tick2: row.try_get("bhops_tick2")?,
			tick3: row.try_get("bhops_tick3")?,
			tick4: row.try_get("bhops_tick4")?,
			tick5: row.try_get("bhops_tick5")?,
			tick6: row.try_get("bhops_tick6")?,
			tick7: row.try_get("bhops_tick7")?,
			tick8: row.try_get("bhops_tick8")?,
		};

		let created_on = row.try_get("created_on")?;

		Ok(Self { id, player, server, time, bhop_stats, created_on })
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TimeSpent {
	/// How many seconds the player actively did something.
	#[serde(with = "crate::serde::duration::as_secs")]
	#[schema(value_type = u16)]
	pub active: Duration,

	/// How many seconds the player was in spectator mode.
	#[serde(with = "crate::serde::duration::as_secs")]
	#[schema(value_type = u16)]
	pub spectating: Duration,

	/// How many seconds the player did nothing.
	#[serde(with = "crate::serde::duration::as_secs")]
	#[schema(value_type = u16)]
	pub afk: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BhopStats {
	pub perfs: u16,
	pub tick0: u16,
	pub tick1: u16,
	pub tick2: u16,
	pub tick3: u16,
	pub tick4: u16,
	pub tick5: u16,
	pub tick6: u16,
	pub tick7: u16,
	pub tick8: u16,
}
