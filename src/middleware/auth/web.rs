use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::{RoleFlags, Session};
use crate::middleware::{Error, Result};

/// Middleware for authenticating users who logged in with Steam.
#[tracing::instrument(skip(request, next))]
pub async fn layer<const REQUIRED_FLAGS: u32>(
	session: Session,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let required_flags = RoleFlags(REQUIRED_FLAGS);

	if !session.role_flags.contains(required_flags) {
		return Err(Error::InsufficientPermissions { required_flags });
	}

	request.extensions_mut().insert(session);

	Ok(next.run(request).await)
}
