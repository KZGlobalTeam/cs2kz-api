use axum::extract::FromRequestParts;
use serde::Deserialize;

mod rejection;
pub use rejection::QueryRejection;

/// An [extractor] for URI query parameters.
///
/// [extractor]: axum::extract
/// [handlers]: axum::handler
#[derive(Debug)]
pub struct Query<T>(pub T)
where
    T: for<'de> Deserialize<'de>;

impl<S, T> FromRequestParts<S> for Query<T>
where
    S: Send + Sync,
    T: for<'de> Deserialize<'de>,
{
    type Rejection = QueryRejection<T>;

    #[tracing::instrument(level = "trace", skip_all, err(level = "debug"))]
    async fn from_request_parts(
        request: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let query = request.uri.query().unwrap_or_default();

        serde_html_form::from_str(query)
            .map(Self)
            .map_err(QueryRejection::new)
    }
}
