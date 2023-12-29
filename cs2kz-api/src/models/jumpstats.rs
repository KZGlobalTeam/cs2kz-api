//! This module holds types related to KZ players.

use chrono::{DateTime, Utc};
use cs2kz::{Jumpstat, Mode, SteamID, Style};
use semver::Version;
use serde::Serialize;
use sqlx::mysql::MySqlRow;
use sqlx::types::Decimal;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use super::players::Player;
use super::servers::ServerSummary;

/// A jumpstat.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({
  "id": 1,
  "kind": "longjump",
  "distance": 269.7274,
  "mode": "kz_classic",
  "style": "normal",
  "player": {
    "steam_id": "STEAM_1:1:161178172",
    "name": "AlphaKeks"
  },
  "server": {
    "id": 1,
    "name": "Alpha's KZ"
  },
  "created_on": "2023-12-10T10:41:01Z"
}))]
pub struct JumpstatResponse {
	id: u64,
	kind: Jumpstat,
	mode: Mode,
	style: Style,
	strafes: u8,
	distance: Decimal,
	sync: Decimal,
	pre: Decimal,
	max: Decimal,
	overlap: Decimal,
	bad_air: Decimal,
	dead_air: Decimal,
	height: Decimal,
	airpath: Decimal,
	deviation: Decimal,
	average_width: Decimal,
	airtime: Decimal,
	player: Player,
	server: ServerSummary,

	#[schema(value_type = String)]
	plugin_version: Version,

	created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for JumpstatResponse {
	fn from_row(row: &'_ MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let kind = row
			.try_get::<u8, _>("kind")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let mode = row
			.try_get::<u8, _>("mode_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let style = row
			.try_get::<u8, _>("style_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let strafes = row.try_get("strafes")?;
		let distance = row.try_get("distance")?;
		let sync = row.try_get("sync")?;
		let pre = row.try_get("pre")?;
		let max = row.try_get("max")?;
		let overlap = row.try_get("overlap")?;
		let bad_air = row.try_get("bad_air")?;
		let dead_air = row.try_get("dead_air")?;
		let height = row.try_get("height")?;
		let airpath = row.try_get("airpath")?;
		let deviation = row.try_get("deviation")?;
		let average_width = row.try_get("average_width")?;
		let airtime = row.try_get("airtime")?;

		let player_id = row.try_get("player_id")?;
		let steam_id =
			SteamID::from_u32(player_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let player_name = row.try_get("player_name")?;
		let player = Player { steam_id, name: player_name };
		let server_id = row.try_get("server_id")?;
		let server_name = row.try_get("server_name")?;
		let server = ServerSummary { id: server_id, name: server_name };
		let plugin_version = row
			.try_get::<&str, _>("plugin_version")?
			.parse()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let created_on = row.try_get("created_on")?;

		Ok(Self {
			id,
			kind,
			mode,
			style,
			strafes,
			distance,
			sync,
			pre,
			max,
			overlap,
			bad_air,
			dead_air,
			height,
			airpath,
			deviation,
			average_width,
			airtime,
			player,
			server,
			plugin_version,
			created_on,
		})
	}
}
