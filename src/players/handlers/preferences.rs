//! Handlers for the `/players/{player}/preferences` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::{PlayerIdentifier, SteamID};
use serde_json::Value as JsonValue;
use sqlx::types::Json as SqlJson;
use sqlx::QueryBuilder;
use tracing::debug;

use crate::auth::{self, Jwt};
use crate::responses::{self, NoContent};
use crate::{Error, Result, State};

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

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  put,
  path = "/players/{steam_id}/preferences",
  tag = "Players",
  security(("CS2 Server" = [])),
  params(SteamID),
  request_body = JsonValue,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn put(
	state: &State,
	Jwt { payload: server, .. }: Jwt<auth::Server>,
	Path(steam_id): Path<SteamID>,
	Json(preferences): Json<JsonValue>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  preferences = ?
		WHERE
		  id = ?
		"#,
		SqlJson(&preferences),
		steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID"));
	}

	transaction.commit().await?;

	debug!(%steam_id, ?preferences, "updated player preferences");

	Ok(NoContent)
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use cs2kz::SteamID;
	use serde_json::{json, Value as JsonValue};
	use uuid::Uuid;

	#[crate::test]
	async fn update_preferences(ctx: &Context) {
		let steam_id = SteamID::from_u64(76561198282622073_u64)?;
		let url = ctx.url(format_args!("/players/{steam_id}/preferences"));
		let jwt = ctx.auth_server(Duration::from_secs(60 * 60))?;
		let preferences = json!({ "funny_test": ctx.test_id });
		let response = ctx
			.http_client
			.put(url.clone())
			.header("Authorization", format!("Bearer {jwt}"))
			.json(&preferences)
			.send()
			.await?;

		assert_eq!(response.status(), 204);

		let response = ctx.http_client.get(url).send().await?;

		assert_eq!(response.status(), 200);

		let mut preferences = response.json::<JsonValue>().await?;
		let funny_test = serde_json::from_value::<Uuid>(preferences["funny_test"].take())?;

		assert_eq!(funny_test, ctx.test_id);
	}
}
