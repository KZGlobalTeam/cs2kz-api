//! HTTP handlers for this service.

use axum::extract::{Extension, State};
use axum::{routing, Router};
use tower::ServiceBuilder;

use super::{
	Error,
	FetchPluginVersionRequest,
	FetchPluginVersionResponse,
	FetchPluginVersionsRequest,
	FetchPluginVersionsResponse,
	PluginService,
	PluginVersionIdentifier,
	SubmitPluginVersionRequest,
	SubmitPluginVersionResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::api_key::ApiKeyLayer;
use crate::services::auth::ApiKey;

impl From<PluginService> for Router
{
	fn from(svc: PluginService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(ApiKeyLayer::new("gh-publish-plugin-versions", svc.database.clone()));

		Router::new()
			.route("/versions", routing::get(get_versions))
			.route("/versions", routing::post(submit_version).route_layer(auth))
			.route("/versions/:version", routing::get(get_version))
			.with_state(svc)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/plugin/versions", tag = "Plugin", params(FetchPluginVersionsRequest))]
async fn get_versions(
	State(svc): State<PluginService>,
	Query(req): Query<FetchPluginVersionsRequest>,
) -> Result<FetchPluginVersionsResponse, ProblemDetails>
{
	let res = svc.fetch_versions(req).await?;

	if res.versions.is_empty() {
		Err(Error::VersionDoesNotExist)?;
	}

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/plugin/versions", tag = "Plugin", security(("API Key" = [])))]
async fn submit_version(
	Extension(key): Extension<ApiKey>,
	State(svc): State<PluginService>,
	Json(req): Json<SubmitPluginVersionRequest>,
) -> Result<SubmitPluginVersionResponse, ProblemDetails>
{
	let res = svc.submit_version(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/plugin/versions/{version}", tag = "Plugin", params(
  ("version" = str, Path, description = "a plugin version identifier"),
))]
async fn get_version(
	State(svc): State<PluginService>,
	Path(ident): Path<PluginVersionIdentifier>,
) -> Result<FetchPluginVersionResponse, ProblemDetails>
{
	let req = FetchPluginVersionRequest { ident };
	let res = svc
		.fetch_version(req)
		.await?
		.ok_or(Error::VersionDoesNotExist)?;

	Ok(res)
}
