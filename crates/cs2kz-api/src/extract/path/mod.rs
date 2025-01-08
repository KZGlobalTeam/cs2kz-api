use std::convert::Infallible;

use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use serde::Deserialize;

mod rejection;
pub use rejection::PathRejection;

/// An [extractor] for URI path parameters.
///
/// [extractor]: axum::extract
/// [handlers]: axum::handler
#[derive(Debug)]
pub struct Path<T>(pub T)
where
    T: for<'de> Deserialize<'de>;

impl<S, T> FromRequestParts<S> for Path<T>
where
    S: Send + Sync,
    T: for<'de> Deserialize<'de> + Send,
{
    type Rejection = PathRejection<T>;

    #[tracing::instrument(level = "trace", skip_all, err(level = "debug"))]
    async fn from_request_parts(
        request: &mut http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        <axum::extract::Path<T> as FromRequestParts<S>>::from_request_parts(request, state)
            .await
            .map(|axum::extract::Path(value)| Self(value))
            .map_err(PathRejection::new)
    }
}

impl<S, T> OptionalFromRequestParts<S> for Path<T>
where
    S: Send + Sync,
    T: for<'de> Deserialize<'de> + Send,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        request: &mut http::request::Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(<Self as FromRequestParts<S>>::from_request_parts(request, state)
            .await
            .ok())
    }
}
