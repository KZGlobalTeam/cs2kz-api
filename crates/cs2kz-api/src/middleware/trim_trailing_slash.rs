// adapted from https://docs.rs/tower-http/0.6.2/src/tower_http/normalize_path.rs

use std::task::Poll;

use http::uri::Builder as UriBuilder;
use tower::layer::layer_fn;

/// Returns a [`tower::Layer`] for trimming trailing slashes from request URIs.
pub fn layer<S, ReqBody>() -> impl tower::Layer<S, Service = TrimTrailingSlash<S>> + Clone
where
    S: tower::Service<http::Request<ReqBody>>,
{
    layer_fn(TrimTrailingSlash::new)
}

/// A middleware to trim trailing `/`s from request URIs.
#[derive(Clone)]
pub struct TrimTrailingSlash<S> {
    service: S,
}

impl<S> TrimTrailingSlash<S> {
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S, ReqBody> tower::Service<http::Request<ReqBody>> for TrimTrailingSlash<S>
where
    S: tower::Service<http::Request<ReqBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future {
        let (mut parts, body) = request.into_parts();

        macro call_service() {
            self.service.call(http::Request::from_parts(parts, body))
        }

        if !parts.uri.path().ends_with('/') {
            return call_service!();
        }

        // The HTML document we serve here uses relative paths.
        // These break if the URI doesn't have a trailing slash.
        if cfg!(not(feature = "production")) && parts.uri.path().starts_with("/docs/swagger-ui") {
            return call_service!();
        }

        let mut new_path_and_query = format!("/{}", parts.uri.path().trim_matches('/'));

        if let Some(query) = parts
            .uri
            .path_and_query()
            .and_then(|path_and_query| path_and_query.query())
        {
            new_path_and_query.push('?');
            new_path_and_query.push_str(query);
        }

        parts.uri = UriBuilder::from(parts.uri)
            .path_and_query(new_path_and_query)
            .build()
            .unwrap();

        call_service!()
    }
}
