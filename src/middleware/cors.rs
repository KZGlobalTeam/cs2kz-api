//! This module contains configuration presets for [CORS] middleware.
//!
//! [CORS]: https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS

use std::time::Duration;

use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, MaxAge};

/// Returns a permissive CORS middleware suitable for `GET` endpoints.
pub fn permissive() -> CorsLayer
{
	CorsLayer::new()
		.allow_credentials(false)
		.allow_headers(AllowHeaders::any())
		.allow_methods([http::Method::OPTIONS, http::Method::GET])
		.allow_origin(AllowOrigin::any())
		.max_age(MaxAge::exact(Duration::MAX))
}

/// Returns a CORS middleware suitable for endpoints used by [the dashboard].
///
/// [the dashboard]: https://github.com/KZGlobalTeam/cs2kz-api-dashboard
pub fn dashboard(methods: impl Into<AllowMethods>) -> CorsLayer
{
	CorsLayer::new()
		.allow_credentials(true)
		.allow_headers([http::header::AUTHORIZATION, http::header::CONTENT_TYPE])
		.allow_methods(methods)
		.allow_origin(if cfg!(feature = "production") {
			AllowOrigin::exact(http::HeaderValue::from_static("https://dashboard.cs2kz.org"))
		} else {
			AllowOrigin::predicate(is_localhost)
		})
		.max_age(MaxAge::exact(Duration::MAX))
}

#[tracing::instrument(level = "trace", ret(level = "debug"), skip_all, fields(
	?origin,
	method = %req.method,
	uri = %req.uri,
	headers = ?req.headers,
	rejection_reason = tracing::field::Empty,
))]
fn is_localhost(origin: &http::HeaderValue, req: &http::request::Parts) -> bool
{
	let Ok(origin) = origin.to_str() else {
		tracing::Span::current().record("rejection_reason", "origin is not utf-8");
		return false;
	};

	let Ok(origin) = origin.parse::<http::Uri>() else {
		tracing::Span::current().record("rejection_reason", "origin is not a valid uri");
		return false;
	};

	if origin.scheme().is_none() {
		tracing::Span::current().record("rejection_reason", "origin is not an http(s) uri");
		return false;
	}

	let Some("localhost" | "127.0.0.1") = origin.host() else {
		tracing::Span::current().record("rejection_reason", "origin is not localhost");
		return false;
	};

	tracing::warn!("allowing sensitive request from localhost");
	true
}
