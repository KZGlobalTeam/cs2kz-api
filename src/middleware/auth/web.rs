use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::permissions::Permissions;
use crate::auth::Session;
use crate::middleware::{Error, Result};

/// Middleware for authenticating users who logged in with Steam.
#[tracing::instrument(skip(request, next))]
pub async fn layer<const REQUIRED: u64>(
	session: Session,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	let required = Permissions(REQUIRED);

	if !session.permissions.contains(required) {
		return Err(Error::InsufficientPermissions { required });
	}

	request.extensions_mut().insert(session);

	Ok(next.run(request).await)
}
