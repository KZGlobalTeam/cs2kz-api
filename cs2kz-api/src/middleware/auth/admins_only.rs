use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::{Error, Result, State};

/// Verifies a request coming from a Global Team member.
#[tracing::instrument(skip_all, ret, err(Debug))]
pub async fn verify_admin(_state: State, _request: Request, _next: Next) -> Result<Response> {
	// TODO
	Err(Error::Unauthorized)
}
