// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Health", context_path = "/api/v0", path = "/health", responses(
	(status = 200, content_type = "text/plain", description = "The API is healthy.")
))]
pub async fn health() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
