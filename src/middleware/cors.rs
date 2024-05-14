//! CORS middlewares.

use axum::http::{header, request, HeaderValue, Method};
use tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};
use url::Url;

/// Creates a permissive CORS layer that allows `GET` requests.
pub fn permissive() -> CorsLayer {
	CorsLayer::permissive().allow_methods([Method::GET])
}

/// Creates a CORS layer that allows requests of the given `methods` from the dashboard.
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

/// Checks if an incoming request came from localhost, ignoring the port.
fn is_localhost(origin: &HeaderValue, _request: &request::Parts) -> bool {
	let Ok(origin) = origin.to_str() else {
		return false;
	};

	let Ok(origin) = Url::parse(origin) else {
		return false;
	};

	if !matches!(origin.scheme(), "http" | "https") {
		return false;
	}

	if !matches!(origin.host_str(), Some("127.0.0.1" | "localhost")) {
		return false;
	}

	true
}
