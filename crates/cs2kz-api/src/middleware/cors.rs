use headers::HeaderMapExt;
use http::{HeaderValue, Method, Uri, header, request};
use tower_http::cors::{AllowCredentials, AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use crate::runtime::{self, Environment};

const KNOWN_HOSTS: &[&str] = &["https://cs2kz.org", "https://dashboard.cs2kz.org"];
const STAGING_HOSTS: &[&str] = &["https://staging.dashboard.cs2kz.org"];
const LOCAL_HOSTS: &[&str] = &["0.0.0.0", "127.0.0.1", "::", "::1", "localhost"];

pub fn layer() -> CorsLayer {
    CorsLayer::new()
        .allow_credentials(AllowCredentials::predicate(allow_credentials))
        .allow_headers(AllowHeaders::mirror_request())
        .allow_methods(AllowMethods::mirror_request())
        .allow_origin(AllowOrigin::predicate(allow_origin))
        .expose_headers([header::COOKIE])
}

fn allow_credentials(_header: &HeaderValue, request: &request::Parts) -> bool {
    match request.method {
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE => true,
        Method::GET => request.uri.path().starts_with("/auth"),
        Method::OPTIONS => request
            .headers
            .typed_get::<headers::AccessControlRequestMethod>()
            .map(Method::from)
            .is_some_and(|method| {
                matches!(method, Method::POST | Method::PUT | Method::PATCH | Method::DELETE)
            }),
        _ => false,
    }
}

fn allow_origin(header: &HeaderValue, request: &request::Parts) -> bool {
    if !allow_credentials(header, request) {
        // request isn't sensitive
        return true;
    }

    let Ok(origin) = header.to_str() else {
        return false;
    };

    match runtime::environment() {
        Environment::Local => origin
            .parse::<Uri>()
            .is_ok_and(|uri| uri.host().is_some_and(|host| LOCAL_HOSTS.contains(&host))),
        Environment::Staging => STAGING_HOSTS.contains(&origin),
        Environment::Production => KNOWN_HOSTS.contains(&origin),
    }
}
