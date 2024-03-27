//! CORS middlewares.

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowMethods, CorsLayer};

/// balls
pub fn permissive() -> CorsLayer {
	CorsLayer::permissive().allow_methods([Method::GET])
}

/// balls
pub fn dashboard(methods: impl Into<AllowMethods>) -> CorsLayer {
	CorsLayer::new()
		.allow_methods(methods)
		.allow_credentials(true)
		.allow_headers([header::CONTENT_TYPE])
		.allow_origin(if cfg!(feature = "production") {
			HeaderValue::from_static("https://dashboard.cs2.kz")
		} else {
			HeaderValue::from_static("http://127.0.0.1")
		})
}
