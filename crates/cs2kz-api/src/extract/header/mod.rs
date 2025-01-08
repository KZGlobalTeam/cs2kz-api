use axum::extract::FromRequestParts;
use headers::{Header as IsHeader, HeaderMapExt};

mod rejection;
pub use rejection::HeaderRejection;

/// An [extractor] for [request headers].
///
/// [extractor]: axum::extract
/// [request headers]: http::Request::headers
#[derive(Debug)]
pub struct Header<H: IsHeader>(pub H);

impl<S, H> FromRequestParts<S> for Header<H>
where
    S: Send + Sync,
    H: IsHeader,
{
    type Rejection = HeaderRejection<H>;

    #[tracing::instrument(level = "trace", skip_all, err(level = "debug"))]
    async fn from_request_parts(
        request: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        match request.headers.typed_try_get::<H>() {
            Ok(Some(header)) => Ok(Self(header)),
            Ok(None) => Err(HeaderRejection::missing()),
            Err(error) => Err(HeaderRejection::parse(error)),
        }
    }
}
