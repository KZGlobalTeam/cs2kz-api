//! Authentication middleware.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::authentication;
use crate::authorization::AuthorizeSession;

/// Extracts an [`authentication::Session`] from the request and inserts it into the request's
/// extensions, and then returns it to extend it.
#[tracing::instrument(level = "debug", name = "middleware::auth", skip(request, next))]
pub async fn layer<A>(
	session: authentication::Session<A>,
	mut request: Request,
	next: Next,
) -> (authentication::Session<A>, Response)
where
	A: AuthorizeSession,
{
	tracing::debug!("inserting session into request extensions");
	request.extensions_mut().insert(session.clone());

	(session, next.run(request).await)
}

/// Creates a middleware for session authentication.
///
/// # Example
///
/// ```rust,ignore
/// let auth = session_auth!(HasPermissions<{ Permissions::ADMIN.value() }>, state.clone());
/// let routes = Router::new()
///     // ...
///     .route_layer(auth)
///     .with_state(state);
/// ```
macro_rules! session_auth {
	($authorization:ty, $state:expr $(,)?) => {
		|| {
			::axum::middleware::from_fn_with_state(
				$state,
				$crate::middleware::auth::layer::<$authorization>,
			)
		}
	};
}

pub(crate) use session_auth;
