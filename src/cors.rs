use axum::http::{header, HeaderValue};
use tower_http::cors::{AllowMethods, CorsLayer};

/// A CORS layer that only allows the provided request methods but is otherwise permissive.
pub fn permissive(methods: impl Into<AllowMethods>) -> CorsLayer {
	CorsLayer::permissive().allow_methods(methods)
}

/// A CORS layer that only allows requests of the specified method, and only localhost and the
/// dashboard as origins.
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
