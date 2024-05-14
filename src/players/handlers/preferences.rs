//! Handlers for the `/players/{player}/preferences` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;
use serde_json::Value as JsonValue;
use sqlx::types::Json as SqlJson;
use sqlx::QueryBuilder;

use crate::openapi::responses;
use crate::{Error, Result, State};

/// Fetch in-game preference settings for a specific player.
///
/// This is used by CS2 servers for keeping settings in sync across multiple servers.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}/preferences",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<responses::Object>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: &State, Path(player): Path<PlayerIdentifier>) -> Result<Json<JsonValue>> {
	let mut query = QueryBuilder::new("SELECT preferences FROM Players WHERE");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push(" id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push(" name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let SqlJson(preferences) = query
		.build_query_scalar::<SqlJson<JsonValue>>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(preferences))
}
