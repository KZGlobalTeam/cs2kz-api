//! Handlers for the `/plugin` route.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::parameters::{Limit, Offset};
use crate::plugin::{CreatedPluginVersion, NewPluginVersion, PluginVersion};
use crate::responses::Created;
use crate::sqlx::QueryBuilderExt;
use crate::{auth, responses, AppState, Error, Result};

/// Query parameters for `GET /plugin`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  params(GetParams),
  responses(
    responses::Ok<PluginVersion>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: AppState,
	Query(GetParams { limit, offset }): Query<GetParams>,
) -> Result<Json<Vec<PluginVersion>>> {
	let mut query = QueryBuilder::new("SELECT * FROM PluginVersions");

	query.push_limits(limit, offset);

	let plugin_versions = query
		.build_query_as::<PluginVersion>()
		.fetch_all(&state.database)
		.await?;

	if plugin_versions.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(plugin_versions))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  security(("API Key" = ["plugin_versions"])),
  request_body = NewPluginVersion,
  responses(
    responses::Created<CreatedPluginVersion>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: AppState,
	key: auth::Key,
	Json(NewPluginVersion { semver, git_revision }): Json<NewPluginVersion>,
) -> Result<Created<Json<CreatedPluginVersion>>> {
	let plugin_version_id = sqlx::query! {
		r#"
		INSERT INTO
		  PluginVersions (semver, git_revision)
		VALUES
		  (?, ?)
		"#,
		semver.to_string(),
		git_revision,
	}
	.execute(&state.database)
	.await
	.map(crate::sqlx::last_insert_id)??;

	Ok(Created(Json(CreatedPluginVersion { plugin_version_id })))
}
