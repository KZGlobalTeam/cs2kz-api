//! Handlers for the `/plugin` route.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use sqlx::QueryBuilder;
use tracing::debug;
use utoipa::IntoParams;

use crate::parameters::{Limit, Offset};
use crate::plugin::{CreatedPluginVersion, NewPluginVersion, PluginVersion};
use crate::responses::Created;
use crate::sqlx::extract::{Connection, Transaction};
use crate::sqlx::{query, QueryBuilderExt, SqlErrorExt};
use crate::{auth, responses, Error, Result};

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

#[tracing::instrument(level = "debug", skip(connection))]
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
	Connection(mut connection): Connection,
	Query(GetParams { limit, offset }): Query<GetParams>,
) -> Result<Json<Vec<PluginVersion>>> {
	let mut query = QueryBuilder::new("SELECT * FROM PluginVersions");

	query.push_limits(limit, offset);

	let plugin_versions = query
		.build_query_as::<PluginVersion>()
		.fetch_all(connection.as_mut())
		.await?;

	if plugin_versions.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(plugin_versions))
}

#[tracing::instrument(level = "debug", skip(transaction))]
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
    responses::Conflict,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	Transaction(mut transaction): Transaction,
	auth::Key(key): auth::Key,
	Json(NewPluginVersion { semver, git_revision }): Json<NewPluginVersion>,
) -> Result<Created<Json<CreatedPluginVersion>>> {
	let latest_version = sqlx::query! {
		r#"
		SELECT
		  semver
		FROM
		  PluginVersions
		ORDER BY
		  created_on DESC
		LIMIT
		  1
		"#
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| row.semver.parse::<semver::Version>())
	.transpose()
	.map_err(|err| Error::internal_server_error("invalid semver in database").with_source(err))?;

	if let Some(version) = latest_version.filter(|version| version >= &semver) {
		return Err(Error::invalid_semver(version));
	}

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
	.execute(transaction.as_mut())
	.await
	.map(query::last_insert_id)
	.map_err(|err| {
		if err.is_duplicate_entry() {
			Error::invalid_plugin_rev()
		} else {
			Error::from(err)
		}
	})??;

	transaction.commit().await?;

	debug!(id = %plugin_version_id, %semver, %git_revision, "created new plugin version");

	Ok(Created(Json(CreatedPluginVersion { plugin_version_id })))
}
