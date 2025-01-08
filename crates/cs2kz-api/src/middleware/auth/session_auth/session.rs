use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use cs2kz::Context;
use cs2kz::time::Timestamp;
use cs2kz::users::sessions::{GetSessionsError, ParseSessionIdError};
use cs2kz::users::{Permissions, UserId};
use ulid::Ulid;

use super::SessionId;
use crate::middleware::auth::session_auth::COOKIE_NAME;
use crate::response::ErrorResponse;

mod inner {
    use super::*;

    #[derive(Debug)]
    pub(super) struct Session {
        pub(super) id: SessionId,
        pub(super) expires_at: Timestamp,
        pub(super) user: User,
        pub(super) should_expire: AtomicBool,
    }
}

#[derive(Clone)]
pub struct Session(Arc<inner::Session>);

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    permissions: Permissions,
}

#[derive(Debug, Display, Error, From)]
pub enum SessionRejection {
    #[display("failed to extract auth cookie")]
    ExtractAuthCookie,

    #[display("failed to parse auth cookie value")]
    ParseSessionId(ParseSessionIdError),

    #[display("something went wrong")]
    GetSession(GetSessionsError),

    #[display("unknown session ID")]
    UnknownSessionId,

    #[display("session has expired")]
    Expired,
}

impl Session {
    pub(super) fn new(id: SessionId, expires_at: Timestamp, user: User) -> Self {
        Self(Arc::new(inner::Session {
            id,
            expires_at,
            user,
            should_expire: AtomicBool::new(false),
        }))
    }

    pub fn id(&self) -> SessionId {
        self.0.id
    }

    pub fn user(&self) -> &User {
        &self.0.user
    }

    pub fn created_at(&self) -> Timestamp {
        Timestamp::from_unix_ms(Ulid::timestamp_ms(self.0.id.as_ref())).expect("timestamp overflow")
    }

    pub fn expires_at(&self) -> Timestamp {
        self.0.expires_at
    }

    pub fn is_expired(&self) -> bool {
        self.0.should_expire.load(atomic::Ordering::SeqCst) || self.0.expires_at < Timestamp::now()
    }

    /// Marks this session for expiration.
    ///
    /// This will cause [`.is_expired()`] to return `false`, and the session to be expired after
    /// the request handler returns.
    ///
    /// Returns whether the session was already marked as expired.
    ///
    /// [`.is_expired()`]: Session::is_expired()
    pub fn expire(&self) -> bool {
        self.0.should_expire.swap(true, atomic::Ordering::SeqCst)
    }
}

impl User {
    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn permissions(&self) -> Permissions {
        self.permissions
    }
}

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
    Context: FromRef<S>,
{
    type Rejection = SessionRejection;

    #[tracing::instrument(level = "debug", skip_all, err(level = "debug"))]
    async fn from_request_parts(
        request: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        if let Some(session) = request.extensions.get::<Session>() {
            trace!(id = %session.id(), "extracted cached session");
            return Ok(session.clone());
        }

        let cx = Context::from_ref(state);
        let session_id = request
            .headers
            .get_all(http::header::COOKIE)
            .into_iter()
            .flat_map(|header| header.to_str())
            .flat_map(Cookie::split_parse_encoded)
            .flatten()
            .find(|cookie| cookie.name() == COOKIE_NAME)
            .map(|cookie| cookie.value().parse::<SessionId>())
            .ok_or(SessionRejection::ExtractAuthCookie)??;

        let session = cs2kz::users::sessions::get(&cx, session_id)
            .await?
            .map(|session| {
                Session::new(session_id, session.expires_at, User {
                    id: session.user_id,
                    permissions: session.user_permissions,
                })
            })
            .ok_or(SessionRejection::UnknownSessionId)?;

        if session.is_expired() {
            return Err(SessionRejection::Expired);
        }

        request.extensions.insert(session.clone());

        Ok(session)
    }
}

impl<S> OptionalFromRequestParts<S> for Session
where
    S: Send + Sync,
    Session: FromRequestParts<S>,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        request: &mut http::request::Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(<Self as FromRequestParts<S>>::from_request_parts(request, state)
            .await
            .ok())
    }
}

impl IntoResponse for SessionRejection {
    fn into_response(self) -> Response {
        match self {
            Self::ExtractAuthCookie
            | Self::ParseSessionId(_)
            | Self::UnknownSessionId
            | Self::Expired => ErrorResponse::unauthorized(),
            Self::GetSession(error) => ErrorResponse::internal_server_error(error),
        }
        .into_response()
    }
}
