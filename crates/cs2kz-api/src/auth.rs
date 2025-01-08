use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{FromRef, State};
use axum::response::{Redirect, Response};
use axum::routing::{self, Router};
use axum_extra::extract::cookie::CookieJar;
use cs2kz::Context;
use cs2kz::access_keys::AccessKey;
use cs2kz::time::Timestamp;
use cs2kz::users::UserId;
use headers::authorization::{Authorization, Bearer};
use steam_openid::VerifyCallbackPayloadError;
use tower::ServiceBuilder;
use tracing::Instrument;
use url::Url;

use crate::config::{CookieConfig, SteamAuthConfig};
use crate::extract::{Header, Json, Query};
use crate::middleware::auth::session_auth;
use crate::middleware::auth::session_auth::Session;
use crate::response::ErrorResponse;
use crate::ws;

const PLAYER_COOKIE_NAME: &str = "kz-player";

#[derive(Clone, FromRef)]
struct LogoutState {
    cx: Context,
    cookie_config: Arc<CookieConfig>,
}

#[derive(Clone, FromRef)]
struct SteamCallbackState {
    cx: Context,
    http_client: reqwest::Client,
    auth_config: Arc<SteamAuthConfig>,
    cookie_config: Arc<CookieConfig>,
}

pub fn router<S>(
    cx: Context,
    auth_config: impl Into<Arc<SteamAuthConfig>>,
    cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Context: FromRef<S>,
{
    let auth_config = auth_config.into();
    let cookie_config = cookie_config.into();

    let logout_state = LogoutState {
        cx: cx.clone(),
        cookie_config: Arc::clone(&cookie_config),
    };

    let steam_callback_state = SteamCallbackState {
        cx,
        http_client: reqwest::Client::new(),
        auth_config: Arc::clone(&auth_config),
        cookie_config,
    };

    Router::new()
        .route("/cs2", routing::any(cs2_server_auth))
        .route("/web", routing::get(get_current_session))
        .route("/web/login", routing::get(user_login).with_state(auth_config))
        .route("/web/logout", routing::get(user_logout).with_state(logout_state))
        .route("/web/steam-callback", routing::get(steam_callback).with_state(steam_callback_state))
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SessionInfo {
    /// When your session was created.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    created_at: Timestamp,

    /// When your session will expire.
    #[schema(value_type = crate::openapi::shims::Timestamp)]
    expires_at: Timestamp,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LoginQuery {
    /// URL you wish to be redirected to after the login process is complete.
    redirect_to: Option<Url>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LogoutQuery {
    /// Expire _all_ sessions, not just your current one.
    #[serde(default)]
    all: bool,
}

/// Establishes a WebSocket connection with the requesting CS2 server.
#[tracing::instrument]
#[utoipa::path(
    get,
    path = "/auth/cs2",
    tag = "CS2 Server Authentication",
    responses(
        (status = 101,),
        (status = 401,),
    ),
)]
async fn cs2_server_auth(
    State(cx): State<Context>,
    Header(Authorization(bearer)): Header<Authorization<Bearer>>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ErrorResponse> {
    let access_key = bearer.token().parse::<AccessKey>().map_err(|err| {
        debug!(%err, "failed to parse access key");
        ErrorResponse::unauthorized()
    })?;

    let server = cs2kz::servers::get_by_access_key(&cx, &access_key)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?
        .ok_or_else(ErrorResponse::unauthorized)?;

    Ok(upgrade.on_upgrade(move |socket| {
        async move {
            info!("server connected");

            let connection = cx.track_future(|cx, shutdown_signal| {
                ws::handle_connection(cx, shutdown_signal, server.id, socket)
            });

            match connection.await {
                Ok(()) => info!("server disconnected"),
                Err(error) => error!(
                    error = &error as &dyn std::error::Error,
                    "failed to handle websocket connection",
                ),
            }
        }
        .instrument(info_span!("cs2_server_connection", %server.id, server.name))
    }))
}

/// Returns information about your current session.
#[tracing::instrument(skip(session))]
#[utoipa::path(
    get,
    path = "/auth/web",
    tag = "User Authentication",
    responses(
        (status = 200, body = SessionInfo),
        (status = 401,),
    ),
)]
async fn get_current_session(session: Session) -> Json<SessionInfo> {
    Json(SessionInfo {
        created_at: session.created_at(),
        expires_at: session.expires_at(),
    })
}

/// Login with Steam.
///
/// This endpoint will redirect you to Steam's login page.
///
/// Afterwards you will be redirected back here, and optionally to another URL if you specify the
/// `redirect_to` query parameter.
#[tracing::instrument(ret(level = "debug"))]
#[utoipa::path(
    get,
    path = "/auth/web/login",
    tag = "User Authentication",
    params(LoginQuery),
    responses(
        (status = 303, description = "Redirect to Steam's login page."),
    ),
)]
async fn user_login(
    State(config): State<Arc<SteamAuthConfig>>,
    Query(LoginQuery { redirect_to }): Query<LoginQuery>,
) -> Result<Redirect, ErrorResponse> {
    let return_to = config
        .public_url
        .join("/auth/web/steam-callback")
        .expect("hard-coded path should be valid");

    let redirect_to = redirect_to
        .as_ref()
        .unwrap_or(&config.redirect_to_after_login);

    steam_openid::login_url(return_to, redirect_to)
        .map(|url| Redirect::to(url.as_str()))
        .map_err(|err| ErrorResponse::internal_server_error(err))
}

/// Expires your current session immediately.
#[tracing::instrument(skip(cx, session), fields(session.id = tracing::field::Empty))]
#[utoipa::path(
    get,
    path = "/auth/web/logout",
    tag = "User Authentication",
    responses(
        (status = 200,),
        (status = 401,),
    ),
)]
async fn user_logout(
    State(LogoutState { cx, cookie_config }): State<LogoutState>,
    session: Option<Session>,
    Query(LogoutQuery { all }): Query<LogoutQuery>,
) -> Result<CookieJar, ErrorResponse> {
    if let Some(session) = session {
        tracing::Span::current().record("session.id", tracing::field::display(session.id()));

        if all {
            cs2kz::users::sessions::expire_all(&cx, session.user().id()).await
        } else {
            cs2kz::users::sessions::expire(&cx, session.id()).await
        }
        .map_err(|err| ErrorResponse::internal_server_error(err))?;
    }

    let player_cookie = cookie_config
        .build_cookie::<false>(PLAYER_COOKIE_NAME, "")
        .removal()
        .build();

    let session_cookie = cookie_config
        .build_cookie::<true>(session_auth::COOKIE_NAME, "")
        .removal()
        .build();

    Ok(CookieJar::new().add(player_cookie).add(session_cookie))
}

#[tracing::instrument(
    skip(cx, http_client, payload),
    fields(payload.userdata),
    ret(level = "debug"),
)]
async fn steam_callback(
    State(SteamCallbackState { cx, http_client, auth_config, cookie_config }): State<
        SteamCallbackState,
    >,
    Query(mut payload): Query<steam_openid::CallbackPayload>,
) -> Result<(CookieJar, Redirect), ErrorResponse> {
    let expected_host = auth_config
        .public_url
        .host()
        .expect("`auth-config.public-url` should have a host");

    let http_client_service = ServiceBuilder::new()
        .map_request(|request| reqwest::Request::try_from(request).expect("uri should be valid"))
        .map_response(http::Response::<reqwest::Body>::from)
        .service(&http_client);

    let steam_id = payload
        .verify(expected_host, http_client_service)
        .await
        .map_err(|err| match err {
            VerifyCallbackPayloadError::HttpClient(error) => {
                ErrorResponse::internal_server_error(error)
            },
            VerifyCallbackPayloadError::HttpRequest(error) => ErrorResponse::bad_gateway(error),
            VerifyCallbackPayloadError::BufferResponseBody { error, response } => {
                debug!(%response.status);
                ErrorResponse::bad_gateway(error)
            },
            VerifyCallbackPayloadError::HostMismatch => ErrorResponse::unauthorized(),
            VerifyCallbackPayloadError::BadStatus { response } => {
                debug!(response.status = %response.status());
                ErrorResponse::unauthorized()
            },
            VerifyCallbackPayloadError::InvalidPayload { response } => {
                debug!(response.status = %response.status(), response.body = ?response.body());
                ErrorResponse::unauthorized()
            },
        })?;

    let user = crate::steam::fetch_user(&http_client, &auth_config.web_api_key, steam_id).await?;
    let new_session = cs2kz::users::sessions::NewSession {
        user_id: UserId::new(user.id),
        user_name: &user.name,
        expires_at: Timestamp::now() + cookie_config.max_age_auth,
    };

    let session_id = cs2kz::users::sessions::login(&cx, new_session)
        .await
        .map_err(|err| ErrorResponse::internal_server_error(err))?;

    let user_json =
        serde_json::to_string(&user).map_err(|err| ErrorResponse::internal_server_error(err))?;

    let player_cookie = cookie_config
        .build_cookie::<false>(PLAYER_COOKIE_NAME, user_json)
        .build();

    let session_cookie = cookie_config
        .build_cookie::<true>(session_auth::COOKIE_NAME, session_id.to_string())
        .build();

    let cookies = CookieJar::new().add(player_cookie).add(session_cookie);

    Ok((cookies, Redirect::to(&payload.userdata)))
}
