use axum::http::{header, HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// A CORS layer that only allows `GET` requests but is otherwise permissive.
pub fn get() -> CorsLayer {
	CorsLayer::permissive().allow_methods(Method::GET)
}

/// A CORS layer that only allows `POST` requests but is otherwise permissive.
pub fn post() -> CorsLayer {
	CorsLayer::permissive().allow_methods(Method::POST)
}

/// A CORS layer that only allows requests of the specified method, and only localhost and the
/// dashboard as origins.
pub fn dashboard(method: Method) -> CorsLayer {
	CorsLayer::new()
		.allow_methods(method)
		.allow_credentials(true)
		.allow_headers([header::CONTENT_TYPE])
		.allow_origin([
			HeaderValue::from_static("http://127.0.0.1"),
			HeaderValue::from_static("https://dashboard.cs2.kz"),
		])
}
