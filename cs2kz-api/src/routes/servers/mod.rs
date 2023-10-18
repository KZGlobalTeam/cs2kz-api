// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::{middleware::server_auth, responses, Result, State},
	axum::{http::StatusCode, Extension},
	rand::Rng,
};

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Servers", context_path = "/api/v0", path = "/servers/refresh_token", responses(
	(status = 201, description = "New token has been generated."),
	(status = 400, response = responses::BadRequest),
	(status = 401, response = responses::Unauthorized),
	(status = 500, response = responses::Database),
))]
pub async fn refresh_token(
	state: State,
	Extension(server_data): Extension<server_auth::ServerData>,
) -> Result<StatusCode> {
	let next_token = rand::thread_rng().gen::<u32>();

	sqlx::query! {
		r#"
		UPDATE
			Servers
		SET
			next_token = ?
		WHERE
			id = ?
		"#,
		next_token,
		server_data.id,
	}
	.execute(state.database())
	.await?;

	// TODO: send UDP request to server to inform it about its new token

	Ok(StatusCode::CREATED)
}
