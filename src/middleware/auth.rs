//! Authentication / Authorization middleware using the [`Session`] extractor.
//!
//! [`Session`]: crate::auth::Session

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::{self, AuthorizeSession};
use crate::Result;

/// Authenticates the incoming request and extends its session.
pub async fn layer<Authorization>(
	session: auth::Session<Authorization>,
	mut request: Request,
	next: Next,
) -> Result<(auth::Session<Authorization>, Response)>
where
	Authorization: AuthorizeSession,
{
	request.extensions_mut().insert(session.clone());

	Ok((session, next.run(request).await))
}

/// macro
macro_rules! session_auth {
	($authorization:ty, $state:expr) => {
		|| {
			::axum::middleware::from_fn_with_state(
				$state,
				$crate::middleware::auth::layer::<$authorization>,
			)
		}
	};
}

pub(crate) use session_auth;
