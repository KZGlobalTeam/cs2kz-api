// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{middleware::auth, responses, Error, Result, State},
	axum::{http::StatusCode, Extension, Json},
	cs2kz::{Mode, SteamID, Style},
	serde::Deserialize,
	sqlx::types::chrono::Utc,
	utoipa::ToSchema,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordRequest {
	map_name: String,
	map_stage: u8,
	map_filesize: u64,
	mode: Mode,
	style: Style,
	steam_id: SteamID,
	teleports: u16,
	ticks: u32,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Records", context_path = "/api/v0", path = "/records", request_body = RecordRequest, responses(
	(status = 201, description = "Record has been inserted into the database."),
	(status = 400, response = responses::BadRequest),
	(status = 401, response = responses::Unauthorized),
	(status = 500, response = responses::Database),
))]
pub async fn create(
	state: State,
	Extension(server_data): Extension<auth::ServerData>,
	Json(record): Json<RecordRequest>,
) -> Result<StatusCode> {
	let course_id = sqlx::query! {
		r#"
		SELECT
			c.id
		FROM
			Courses c
			JOIN Maps m ON m.name = ?
			AND m.filesize = ?
			AND c.stage = ?
		"#,
		record.map_name,
		record.map_filesize,
		record.map_stage,
	}
	.fetch_one(state.database())
	.await
	.map_err(|_| Error::InvalidMap)?
	.id;

	let now = Utc::now();

	sqlx::query! {
		r#"
		INSERT INTO
			Records (
				course_id,
				mode_id,
				style_id,
				player_id,
				server_id,
				teleports,
				ticks,
				created_on
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		course_id,
		record.mode as u8,
		record.style as u8,
		record.steam_id.as_u32(),
		server_data.id,
		record.teleports,
		record.ticks,
		now,
	}
	.execute(state.database())
	.await?;

	Ok(StatusCode::CREATED)
}
