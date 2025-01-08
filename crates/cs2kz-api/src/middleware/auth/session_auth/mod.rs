use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use cs2kz::Context;
use cs2kz::users::sessions::{SessionId, UpdateSessionError};

use self::authorization::AuthorizeSession;
use crate::config::CookieConfig;
use crate::response::ErrorResponse;

pub const COOKIE_NAME: &str = "kz-auth";

pub mod session;
pub use session::Session;

pub mod authorization;

#[tracing::instrument(
    skip(cx, authorization, session, cookies, request, next),
    fields(session.id = %session.id()),
    err(level = "debug"),
)]
pub async fn session_auth(
    State { cx, cookie_config, mut authorization }: State<impl AuthorizeSession>,
    session: Session,
    cookies: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, Rejection> {
    let response = match authorization
        .authorize_session(&mut request, &session)
        .await
    {
        Ok(()) => next.run(request).await,
        Err(rejection) => rejection.into_response(),
    };

    let mut cookie = cookie_config
        .build_cookie::<true>(COOKIE_NAME, session.id().to_string())
        .build();

    if session.is_expired() {
        cookie.make_removal();
        cs2kz::users::sessions::expire(&cx, session.id()).await?;
    } else {
        cs2kz::users::sessions::extend(&cx, session.id(), cookie_config.max_age_auth).await?;
    }

    Ok((cookies.add(cookie), response).into_response())
}

#[derive(Clone)]
pub struct State<A = authorization::Noop> {
    cx: Context,
    cookie_config: Arc<CookieConfig>,
    authorization: A,
}

#[derive(Debug, Display, Error, From)]
#[display("something went wrong")]
pub struct Rejection(UpdateSessionError);

impl State {
    pub fn new(cx: Context, cookie_config: impl Into<Arc<CookieConfig>>) -> Self {
        Self {
            cx,
            cookie_config: cookie_config.into(),
            authorization: authorization::Noop,
        }
    }
}

impl<A> State<A> {
    pub fn authorize_with<NewA: AuthorizeSession>(self, authorization: NewA) -> State<NewA> {
        State {
            cx: self.cx,
            cookie_config: self.cookie_config,
            authorization,
        }
    }

    pub fn map_authorization<NewA: AuthorizeSession>(
        self,
        f: impl FnOnce(A) -> NewA,
    ) -> State<NewA> {
        State {
            cx: self.cx,
            cookie_config: self.cookie_config,
            authorization: f(self.authorization),
        }
    }
}

impl<S, A> FromRequestParts<S> for State<A>
where
    S: Send + Sync,
    Self: FromRef<S>,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_ref(state))
    }
}

impl<A> FromRef<State<A>> for Context {
    fn from_ref(state: &State<A>) -> Self {
        state.cx.clone()
    }
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        ErrorResponse::internal_server_error(self.0).into_response()
    }
}
