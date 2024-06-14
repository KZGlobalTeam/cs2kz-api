//! [CORS] middleware.
//!
//! [CORS]: https://developer.mozilla.org/en-US/docs/Glossary/CORS

use axum::http::{header, request, HeaderValue, Method};
use tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};
use url::Url;

/// Creates a permissive CORS layer, allowing any origins or headers, but only GET requests.
pub fn permissive() -> CorsLayer {
	CorsLayer::permissive().allow_methods([Method::GET])
}

/// Creates a CORS layer for the [dashboard].
///
/// [dashboard]: https://github.com/kzglobalteam/cs2kz-api-dashboard
pub fn dashboard<M>(methods: M) -> CorsLayer
where
	M: Into<AllowMethods>,
{
	CorsLayer::new()
		.allow_methods(methods)
		.allow_credentials(true)
		.allow_headers([header::CONTENT_TYPE])
		.allow_origin(if cfg!(feature = "production") {
			AllowOrigin::exact(HeaderValue::from_static("https://dashboard.cs2kz.org"))
		} else {
			AllowOrigin::predicate(is_localhost)
		})
}

/// Checks if a request is coming from localhost.
#[tracing::instrument(level = "debug", name = "middleware::cors", skip(_request))]
fn is_localhost(origin: &HeaderValue, _request: &request::Parts) -> bool {
	#[allow(clippy::missing_docs_in_private_items)]
	macro_rules! reject {
		($($reason:tt)*) => {
			tracing::debug!("rejecting request because {}", $($reason)*);
			return false;
		};
	}

	let Ok(origin) = origin.to_str() else {
		reject!("origin is not utf-8");
	};

	let Ok(origin) = Url::parse(origin) else {
		reject!("origin is not a URL");
	};

	if !matches!(origin.scheme(), "http" | "https") {
		reject!("origin URL is not http(s)");
	}

	if !matches!(origin.host_str(), Some("127.0.0.1" | "localhost")) {
		reject!("origin host is not localhost");
	}

	tracing::debug!("allowing request from localhost");

	true
}
