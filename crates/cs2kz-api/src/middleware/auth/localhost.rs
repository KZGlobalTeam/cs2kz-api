use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::response::ErrorResponse;
use crate::runtime::{self, Environment};

/// Middleware to check if a given request is coming from localhost.
///
/// This is used for private endpoints like `/metrics` and `/taskdump`.
#[tracing::instrument(skip_all, err(Debug, level = "debug"))]
pub async fn client_is_localhost(request: Request, next: Next) -> Result<Response, ErrorResponse> {
    match runtime::environment() {
        Environment::Local => Ok(next.run(request).await),
        Environment::Staging | Environment::Production => {
            // If there's an X-Real-Ip header, we went through nginx, which means the request came
            // from the outside.
            if request.headers().contains_key("X-Real-Ip") {
                Err(ErrorResponse::not_found())
            } else {
                Ok(next.run(request).await)
            }
        },
    }
}
