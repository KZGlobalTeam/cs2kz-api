use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::response::ErrorResponse;

/// Middleware to check if a given request is coming from localhost.
///
/// This is used for private endpoints like `/metrics` and `/taskdump`.
#[tracing::instrument(skip_all, err(Debug, level = "debug"))]
pub async fn client_is_localhost(request: Request, next: Next) -> Result<Response, ErrorResponse> {
    if request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|addr| addr.ip().is_loopback())
    {
        Ok(next.run(request).await)
    } else {
        Err(ErrorResponse::not_found())
    }
}
