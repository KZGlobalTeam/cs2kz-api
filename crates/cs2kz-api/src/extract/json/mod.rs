use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use mime::Mime;
use serde::{Deserialize, Serialize};

mod rejection;
pub use rejection::JsonRejection;

/// A JSON request/response body.
///
/// This type implements [`FromRequest`] and [`IntoResponse`], so it can be used as an [extractor]
/// and return value from [handlers].
///
/// [extractor]: axum::extract
/// [handlers]: axum::handler
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    S: Send + Sync,
    T: for<'de> Deserialize<'de>,
{
    type Rejection = JsonRejection<T>;

    #[tracing::instrument(level = "trace", skip_all, err(level = "debug"))]
    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        if !has_json_content_type(request.headers()) {
            return Err(JsonRejection::missing_content_type());
        }

        let bytes = Bytes::from_request(request, &())
            .await
            .map_err(JsonRejection::buffer_body)?;

        serde_json::from_slice(&bytes[..])
            .map(Self)
            .map_err(JsonRejection::deserialize)
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let mut response = serde_json::to_vec(&self.0)
            .map(|bytes| Bytes::from(bytes).into_response())
            .unwrap_or_else(|error| panic!("failed to serialize response body: {error}"));

        response.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        );

        response
    }
}

#[tracing::instrument(level = "trace", skip_all, ret)]
fn has_json_content_type(headers: &http::HeaderMap) -> bool {
    let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
        debug!("request headers do not contain a `Content-Type` header");
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        debug!("request headers contain a `Content-Type` header, but it's not UTF-8");
        return false;
    };

    let Ok(mime) = content_type.parse::<Mime>() else {
        debug!("request headers contain a `Content-Type` header, but it's not a valid mime type");
        return false;
    };

    mime.type_() == mime::APPLICATION
        && (mime.subtype() == mime::JSON || mime.suffix() == Some(mime::JSON))
}
