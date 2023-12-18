//! This module holds types related to KZ players.

use chrono::{DateTime, Utc};
use cs2kz::{Jumpstat, Mode, SteamID, Style};
use serde::Serialize;
use sqlx::mysql::MySqlRow;
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
  "mode": "kz_modded",
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
	distance: f64,
	mode: Mode,
	style: Style,
	player: Player,
	server: ServerSummary,
	created_on: DateTime<Utc>,
}

impl FromRow<'_, MySqlRow> for JumpstatResponse {
	fn from_row(row: &'_ MySqlRow) -> Result<Self, sqlx::Error> {
		let id = row.try_get("id")?;
		let kind = row
			.try_get::<u8, _>("type")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let distance = row.try_get("distance")?;

		let mode = row
			.try_get::<u8, _>("mode_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let style = row
			.try_get::<u8, _>("style_id")?
			.try_into()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let player_id = row.try_get("player_id")?;
		let steam_id =
			SteamID::from_u32(player_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let player_name = row.try_get("player_name")?;
		let player = Player { steam_id, name: player_name };
		let server_id = row.try_get("server_id")?;
		let server_name = row.try_get("server_name")?;
		let server = ServerSummary { id: server_id, name: server_name };
		let created_on = row.try_get("created_on")?;

		Ok(Self { id, kind, distance, mode, style, player, server, created_on })
	}
}
