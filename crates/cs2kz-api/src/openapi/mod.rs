use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, LazyLock, OnceLock};

use axum::extract::State;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Router, routing};
use utoipa::openapi::{OpenApi, ServerBuilder};

use crate::config::ServerConfig;
use crate::extract::{Json, Path};
use crate::runtime;

pub mod shims;

static SCHEMA: OnceLock<OpenApi> = OnceLock::new();
static CONFIG: LazyLock<Arc<utoipa_swagger_ui::Config<'static>>> = LazyLock::new(|| {
    let cfg = utoipa_swagger_ui::Config::from("/docs/openapi.json")
        .display_operation_id(true)
        .use_base_layout()
        .display_request_duration(true)
        .filter(true)
        .request_snippets_enabled(true)
        .with_credentials(true);

    Arc::new(cfg)
});

#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "CS2KZ API",
        description = "This is the description :)",
        license(name = "GPL-3.0", url = "https://www.gnu.org/licenses/gpl-3.0.en.html"),
    ),
    external_docs(url = "https://docs.cs2kz.org/api", description = "High-Level documentation"),
    servers(
        (url = "https://api.cs2kz.org", description = "production instance"),
    ),
    modifiers(),
    tags(
        (name = "cs2kz-metamod", description = "used by GitHub Actions in [`KZGlobalTeam/cs2kz-metamod`](https://github.com/KZGlobalTeam/cs2kz-metamod)"),
        (name = "Users"),
        (name = "User Authentication", description = "OpenID 2.0 authentication with Steam"),
        (name = "CS2 Servers", description = "CS2 servers running the cs2kz-metamod plugin"),
        (name = "CS2 Server Authentication"),
        (name = "Players"),
        (name = "Maps"),
        (name = "Jumpstats"),
        (name = "Records"),
        (name = "Player Bans"),
    ),
    components(
        schemas(
            shims::Limit,
            shims::Offset,
            shims::SteamId,
            shims::SteamId64,
            shims::Timestamp,
            shims::GitRevision,
            shims::Checksum,
            shims::Permissions,
            shims::Players_SortBy,
            shims::ServerHost,
            shims::AccessKey,
            shims::MapState,
            shims::CourseFilterState,
            shims::CourseFilterTier,
            shims::Mode,
            shims::Style,
            shims::Styles,
            shims::JumpType,
            shims::BanReason,
            shims::BannedBy,
            shims::Records_SortBy,
            shims::Records_SortOrder,
            crate::players::PlayerIdentifier,
            crate::servers::ServerIdentifier,
            crate::maps::MapIdentifier,
            crate::plugin::PluginVersionIdentifier,
        ),
    ),
    paths(
        crate::plugin::publish_plugin_version,
        crate::plugin::get_plugin_versions,
        crate::plugin::get_plugin_version,

        crate::users::get_users,
        crate::users::get_user,
        crate::users::update_user_email,
        crate::users::delete_user_email,
        crate::users::update_user_permissions,

        crate::auth::cs2_server_auth,
        crate::auth::get_current_session,
        crate::auth::user_login,
        crate::auth::user_logout,

        crate::servers::approve_server,
        crate::servers::get_servers,
        crate::servers::get_server,
        crate::servers::update_server,
        crate::servers::refresh_server_access_key,
        crate::servers::delete_server_access_key,

        crate::players::get_players,
        crate::players::get_player,
        crate::players::get_player_steam_profile,
        crate::players::get_player_preferences,
        crate::players::update_player_preferences,

        crate::maps::approve_map,
        crate::maps::get_maps,
        crate::maps::get_map,
        crate::maps::update_map,

        crate::jumpstats::get_jumpstats,
        crate::jumpstats::get_jumpstat,

        crate::records::get_records,
        crate::records::get_record,

        crate::bans::create_ban,
        crate::bans::get_bans,
        crate::bans::get_ban,
        crate::bans::update_ban,
        crate::bans::delete_ban,
    ),
)]
pub struct Schema;

pub fn schema() -> OpenApi {
    <Schema as utoipa::OpenApi>::openapi()
}

pub(crate) fn router<S>(server_config: impl Into<Arc<ServerConfig>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new()
        .route("/openapi.json", routing::get(serve_openapi_json).with_state(server_config.into()));

    if runtime::environment().is_production() {
        return router;
    }

    router
        .route("/swagger-ui", routing::get(async || Redirect::permanent("/docs/swagger-ui/")))
        .route("/swagger-ui/", routing::get(serve_swagger_ui))
        .route("/swagger-ui/{*rest}", routing::get(serve_swagger_ui))
}

async fn serve_openapi_json(State(config): State<Arc<ServerConfig>>) -> Response {
    let schema = SCHEMA.get_or_init(|| {
        let mut schema = self::schema();

        if runtime::environment().is_local() | runtime::environment().is_staging() {
            let staging_server = ServerBuilder::new()
                .url("https://staging.cs2kz.org")
                .description(Some("staging server"))
                .build();

            schema
                .servers
                .get_or_insert_default()
                .insert(0, staging_server);
        }

        if runtime::environment().is_local() {
            let mut local_addr = config.socket_addr();

            local_addr.set_ip(match local_addr.ip() {
                IpAddr::V4(ipv4) if ipv4.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V6(ipv6) if ipv6.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
                ip => ip,
            });

            let local_server = ServerBuilder::new()
                .url(format!("http://{local_addr}"))
                .description(Some("local dev server"))
                .build();

            schema
                .servers
                .get_or_insert_default()
                .insert(0, local_server);
        }

        schema
    });

    Json(schema).into_response()
}

#[tracing::instrument(ret(level = "debug"))]
async fn serve_swagger_ui(path: Option<Path<String>>) -> Response {
    let tail = match path {
        None => "",
        Some(Path(ref path)) => path.as_str(),
    };

    match utoipa_swagger_ui::serve(tail, Arc::clone(&*CONFIG)) {
        Ok(None) => http::StatusCode::NOT_FOUND.into_response(),
        Ok(Some(file)) => http::Response::builder()
            .header(http::header::CONTENT_TYPE, file.content_type)
            .body(file.bytes.into())
            .unwrap(),
        Err(error) => {
            error!(%error, "failed to serve SwaggerUI file");
            http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        },
    }
}
