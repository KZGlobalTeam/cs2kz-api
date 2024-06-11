//! Authentication / Authorization middleware using the [`Session`] extractor.
//!
//! [`Session`]: crate::authentication::Session

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::authentication;
use crate::authorization::AuthorizeSession;

/// Authenticates the incoming request and extends its session.
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

/// Helper macro for creating auth middleware.
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
