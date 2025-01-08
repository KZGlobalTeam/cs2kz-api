use std::sync::Arc;

use axum::extract::{FromRequestParts, Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cs2kz::Context;
use cs2kz::access_keys::{AccessKey, GetAccessKeyInfoError, ParseAccessKeyError};
use headers::authorization::{Authorization, Bearer};

use crate::extract::Header;
use crate::response::ErrorResponse;

/// Middleware to extract an API key from the request headers.
///
/// These API keys are used by endpoints for internal processes, such as GitHub Actions.
#[tracing::instrument(
    skip(cx, expected_key, request, next),
    fields(expected_key = &*expected_key),
    err(Debug, level = "debug"),
)]
pub async fn access_key(
    State { cx, expected_key }: State,
    Header(Authorization(bearer)): Header<Authorization<Bearer>>,
    request: Request,
    next: Next,
) -> Result<Response, Rejection> {
    let supplied_key = bearer.token().parse::<AccessKey>()?;
    let key_info = cs2kz::access_keys::get(&cx, &supplied_key)
        .await?
        .ok_or(Rejection::KeyNotFound)?;

    if *key_info.name == *expected_key {
        Ok(next.run(request).await)
    } else {
        Err(Rejection::KeyMismatch)
    }
}

#[derive(Clone, FromRequestParts)]
#[from_request(via(axum::extract::State))]
pub struct State {
    cx: Context,

    /// The name of the key we expect to be used.
    expected_key: Arc<str>,
}

impl State {
    pub fn new(cx: Context, expected_key: impl Into<Arc<str>>) -> Self {
        Self { cx, expected_key: expected_key.into() }
    }
}

#[derive(Debug, From)]
pub enum Rejection {
    #[expect(dead_code, reason = "this is information is captured in logs")]
    ParseAuthorizationHeader(ParseAccessKeyError),
    KeyNotFound,
    GetKey(GetAccessKeyInfoError),
    KeyMismatch,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        match self {
            Self::ParseAuthorizationHeader(_)
            | Self::KeyNotFound
            | Self::GetKey(GetAccessKeyInfoError::Expired(_))
            | Self::KeyMismatch => ErrorResponse::unauthorized(),
            Self::GetKey(error) => ErrorResponse::internal_server_error(error),
        }
        .into_response()
    }
}
