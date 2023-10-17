// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{middleware::auth, responses, Result, State},
	axum::{http::StatusCode, Extension, Json},
	cs2kz::{Jumpstat, Mode, SteamID, Style},
	serde::Deserialize,
	sqlx::types::chrono::Utc,
	utoipa::ToSchema,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct JumpstatRequest {
	r#type: Jumpstat,
	distance: f32,
	mode: Mode,
	style: Style,
	steam_id: SteamID,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Jumpstats", context_path = "/api/v0", path = "/jumpstats", request_body = JumpstatRequest, responses(
	(status = 201, description = "Jumpstat has been inserted into the database."),
	(status = 400, response = responses::BadRequest),
	(status = 401, response = responses::Unauthorized),
	(status = 500, response = responses::Database),
))]
pub async fn create(
	state: State,
	Extension(server_data): Extension<auth::ServerData>,
	Json(JumpstatRequest { r#type, distance, mode, style, steam_id }): Json<JumpstatRequest>,
) -> Result<StatusCode> {
	let now = Utc::now();

	sqlx::query! {
		r#"
		INSERT INTO
			Jumpstats (
				`type`,
				distance,
				mode_id,
				style_id,
				player_id,
				server_id,
				created_on
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?)
		"#,
		r#type as u8,
		distance,
		mode as u8,
		style as u8,
		steam_id.as_u32(),
		server_data.id,
		now,
	}
	.execute(state.database())
	.await?;

	Ok(StatusCode::CREATED)
}
