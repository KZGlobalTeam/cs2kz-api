use std::fmt;

use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use utoipa::openapi;

#[derive(Clone)]
pub struct ReplayFile {
    bytes: Bytes,
}

impl ReplayFile {
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self { bytes: bytes.into() }
    }
}

impl fmt::Debug for ReplayFile {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ReplayFile")
            .field("size", &self.bytes.len())
            .finish()
    }
}

impl IntoResponse for ReplayFile {
    fn into_response(self) -> Response {
        self.bytes.into_response()
    }
}

impl utoipa::PartialSchema for ReplayFile {
    fn schema() -> openapi::RefOr<openapi::Schema> {
        openapi::Schema::Object(
            openapi::Object::builder()
                .content_media_type(mime::APPLICATION_OCTET_STREAM.as_ref())
                .format(Some(openapi::SchemaFormat::KnownFormat(
                    openapi::schema::KnownFormat::Binary,
                )))
                .build(),
        )
        .into()
    }
}

impl utoipa::ToSchema for ReplayFile {}
