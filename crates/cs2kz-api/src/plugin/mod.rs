use axum::extract::{FromRef, State};
use axum::handler::Handler;
use axum::routing::{self, MethodRouter, Router};
use cs2kz::Context;
use cs2kz::checksum::Checksum;
use cs2kz::git::GitRevision;
use cs2kz::mode::Mode;
use cs2kz::pagination::{Limit, Offset, Paginated};
use cs2kz::plugin::{PluginVersionId, PublishPluginVersionError};
use cs2kz::styles::Style;
use cs2kz::time::Timestamp;

use crate::config;
use crate::extract::{Json, Path, Query};
use crate::middleware::auth;
use crate::response::{Created, ErrorResponse};

mod version_identifier;
pub use version_identifier::PluginVersionIdentifier;

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewPluginVersion {
    /// A SemVer version.
    #[schema(value_type = str, example = "1.23.456-dev")]
    version: semver::Version,

    /// The git revision associated with the release commit / tag.
    #[schema(value_type = crate::openapi::shims::GitRevision)]
    git_revision: GitRevision,

    /// Checksum of the plugin binary on Linux
    #[schema(value_type = crate::openapi::shims::Checksum)]
    linux_checksum: Checksum,

    /// Checksum of the plugin binary on Windows
    #[schema(value_type = crate::openapi::shims::Checksum)]
    windows_checksum: Checksum,

    /// Whether this release invalidates all previous releases
    is_cutoff: bool,

    modes: Vec<NewMode>,

    styles: Vec<NewStyle>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewMode {
    #[schema(value_type = crate::openapi::shims::Mode)]
    mode: Mode,

    #[schema(value_type = crate::openapi::shims::Checksum)]
    linux_checksum: Checksum,

    #[schema(value_type = crate::openapi::shims::Checksum)]
    windows_checksum: Checksum,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewStyle {
    #[schema(value_type = crate::openapi::shims::Style)]
    style: Style,

    #[schema(value_type = crate::openapi::shims::Checksum)]
    linux_checksum: Checksum,

    #[schema(value_type = crate::openapi::shims::Checksum)]
    windows_checksum: Checksum,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PluginVersion {
    #[schema(value_type = u16, minimum = 1)]
    pub id: PluginVersionId,

    /// A SemVer version.
    #[schema(value_type = str, example = "1.23.456-dev")]
    pub version: semver::Version,

    /// The git revision associated with the release commit / tag of this version.
    #[schema(value_type = crate::openapi::shims::GitRevision)]
    pub git_revision: GitRevision,

    /// When this version was published.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    pub published_at: Timestamp,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PublishedPluginVersion {
    #[schema(value_type = u16, minimum = 1)]
    plugin_version_id: PluginVersionId,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPluginVersionsQuery {
    /// Only include versions that meet this SemVer requirement.
    #[serde(rename = "version")]
    #[param(value_type = Option<String>, example = "^1.0.0")]
    version_req: Option<semver::VersionReq>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Limit)]
    limit: Limit<250, 10>,

    #[serde(default)]
    #[param(value_type = crate::openapi::shims::Offset)]
    offset: Offset,
}

pub fn router<S>(cx: Context, access_keys: &config::AccessKeys) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let auth = axum::middleware::from_fn_with_state(
        auth::access_key::State::new(cx.clone(), &*access_keys.cs2kz_metamod_release_key),
        auth::access_key,
    );

    Router::new()
        .route(
            "/versions",
            MethodRouter::new()
                .post(publish_plugin_version.layer(auth))
                .get(get_plugin_versions),
        )
        .route("/versions/{version}", routing::get(get_plugin_version))
}

/// Notifies the API that a new version of cs2kz-metamod has been released.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    post,
    path = "/plugin/versions",
    tag = "cs2kz-metamod",
    request_body = NewPluginVersion,
    responses(
        (status = 201, body = PublishedPluginVersion),
        (status = 401,),
        (status = 409, description = "The submitted version already exists or is older than the \
                                      current latest version."),
        (status = 422, description = "invalid request body"),
    ),
)]
async fn publish_plugin_version(
    State(cx): State<Context>,
    Json(NewPluginVersion {
        version,
        git_revision,
        linux_checksum,
        windows_checksum,
        is_cutoff,
        modes,
        styles,
    }): Json<NewPluginVersion>,
) -> Result<Created<PublishedPluginVersion>, ErrorResponse> {
    let plugin_version = cs2kz::plugin::NewPluginVersion {
        version: &version,
        git_revision: &git_revision,
        linux_checksum: &linux_checksum,
        windows_checksum: &windows_checksum,
        is_cutoff,
        modes: modes.iter().map(|mode| cs2kz::plugin::NewMode {
            mode: mode.mode,
            linux_checksum: &mode.linux_checksum,
            windows_checksum: &mode.windows_checksum,
        }),
        styles: styles.iter().map(|style| cs2kz::plugin::NewStyle {
            style: style.style,
            linux_checksum: &style.linux_checksum,
            windows_checksum: &style.windows_checksum,
        }),
    };

    cs2kz::plugin::publish_version(&cx, plugin_version)
        .await
        .map(|plugin_version_id| Created(PublishedPluginVersion { plugin_version_id }))
        .map_err(|err| match err {
            PublishPluginVersionError::VersionAlreadyPublished => {
                ErrorResponse::plugin_version_already_exists()
            },
            PublishPluginVersionError::VersionOlderThanLatest { latest } => {
                ErrorResponse::outdated_plugin_version(&latest)
            },
            PublishPluginVersionError::Database(error) => {
                ErrorResponse::internal_server_error(error)
            },
        })
}

/// Returns the latest cs2kz-metamod releases.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/plugin/versions",
    tag = "cs2kz-metamod",
    params(GetPluginVersionsQuery),
    responses(
        (status = 200, body = crate::openapi::shims::Paginated<PluginVersion>),
        (status = 400, description = "invalid query parameters"),
    ),
)]
async fn get_plugin_versions(
    State(cx): State<Context>,
    Query(GetPluginVersionsQuery { version_req, limit, offset }): Query<GetPluginVersionsQuery>,
) -> Result<Json<Paginated<Vec<PluginVersion>>>, ErrorResponse> {
    let params = cs2kz::plugin::GetPluginVersionsParams {
        version_req: version_req.as_ref(),
        limit,
        offset,
    };

    let plugin_versions = cs2kz::plugin::get_versions(&cx, params)
        .await
        .map(|paginated| paginated.map_values(PluginVersion::from))
        .map_err(|err| ErrorResponse::internal_server_error(err))?;

    Ok(Json(plugin_versions))
}

/// Returns metadata about the release of a specific cs2kz-metamod version.
#[tracing::instrument(skip(cx))]
#[utoipa::path(
    get,
    path = "/plugin/versions/{version}",
    tag = "cs2kz-metamod",
    params(("version" = PluginVersionIdentifier, Path, description = "a SemVer version or git \
                                                                      revision")),
    responses(
        (status = 200, body = PluginVersion),
        (status = 400, description = "invalid path parameter"),
        (status = 404,),
    ),
)]
async fn get_plugin_version(
    State(cx): State<Context>,
    Path(version_identifier): Path<PluginVersionIdentifier>,
) -> Result<Json<PluginVersion>, ErrorResponse> {
    let plugin_version = match version_identifier {
        PluginVersionIdentifier::SemVer(ref version) => {
            cs2kz::plugin::get_version(&cx, version).await
        },
        PluginVersionIdentifier::GitRevision(ref git_revision) => {
            cs2kz::plugin::get_version_by_git_revision(&cx, git_revision).await
        },
    }
    .map_err(|err| ErrorResponse::internal_server_error(err))?
    .ok_or_else(ErrorResponse::not_found)?;

    Ok(Json(plugin_version.into()))
}

impl From<cs2kz::plugin::PluginVersion> for PluginVersion {
    fn from(plugin_version: cs2kz::plugin::PluginVersion) -> Self {
        Self {
            id: plugin_version.id,
            version: plugin_version.version,
            git_revision: plugin_version.git_revision,
            published_at: plugin_version.published_at,
        }
    }
}
