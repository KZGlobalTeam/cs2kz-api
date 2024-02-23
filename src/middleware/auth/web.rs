use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::sessions::Admin;
use crate::auth::Session;

/// Middleware for authenticating users who logged in with Steam.
///
/// This layer is "necessary" because we need to return the [`Session`] with the response so it can
/// be extended automatically. If we took [`Session`] as an extractor in every handler, we would
///    1) potentially take an unused argument (confusing)
///    2) have to return it as well
/// This layer ensures the session is extended, but also available as an [`Extension`] if access to
/// it from a handler is necessary. [`Session`] will check the request extensions for an instance
/// of itself in its `FromRequestParts` implementation to prevent duplicate extraction.
///
/// [`Extension`]: axum::Extension
#[tracing::instrument(skip(request, next))]
pub async fn layer<const REQUIRED_FLAGS: u32>(
	session: Session<Admin<REQUIRED_FLAGS>>,
	mut request: Request,
	next: Next,
) -> (Session<Admin<REQUIRED_FLAGS>>, Response) {
	request.extensions_mut().insert(session.clone());

	(session, next.run(request).await)
}
