use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

use axum::extract::rejection::BytesRejection;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::response::ErrorResponse;

pub struct JsonRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    reason: Reason,
    _marker: PhantomData<T>,
}

#[derive(Debug, Display, Error)]
enum Reason {
    #[display("missing `Content-Type: application/json` header")]
    MissingContentType,

    #[display("request body too large")]
    BufferBody(BytesRejection),

    #[display("{_0}")]
    Deserialize(serde_json::Error),
}

impl<T> JsonRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn new(reason: Reason) -> Self {
        Self { reason, _marker: PhantomData }
    }

    pub(super) fn missing_content_type() -> Self {
        Self::new(Reason::MissingContentType)
    }

    pub(super) fn buffer_body(rejection: BytesRejection) -> Self {
        Self::new(Reason::BufferBody(rejection))
    }

    pub(super) fn deserialize(error: serde_json::Error) -> Self {
        Self::new(Reason::Deserialize(error))
    }
}

impl<T> fmt::Debug for JsonRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("JsonRejection")
            .field("reason", &self.reason)
            .finish()
    }
}

impl<T> fmt::Display for JsonRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "failed to deserialize request body")?;

        if cfg!(not(feature = "production")) {
            write!(fmt, " of type `{}`", type_name::<T>())?;
        }

        write!(fmt, ": {}", self.reason)
    }
}

impl<T> std::error::Error for JsonRejection<T> where T: for<'de> Deserialize<'de> {}

impl<T> IntoResponse for JsonRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn into_response(self) -> Response {
        #[derive(serde::Serialize)]
        struct JsonError {
            #[serde(rename = "type")]
            kind: &'static str,
            line: usize,
            column: usize,
            detail: String,
        }

        match self.reason {
            Reason::MissingContentType => ErrorResponse::missing_header::<headers::ContentType>(),
            Reason::BufferBody(_) => ErrorResponse::failed_to_buffer_body(),
            Reason::Deserialize(error) => ErrorResponse::invalid_request_body(|details| {
                details.add_extension("json_error", &JsonError {
                    kind: match error.classify() {
                        serde_json::error::Category::Io | serde_json::error::Category::Eof => "eof",
                        serde_json::error::Category::Syntax => "syntax",
                        serde_json::error::Category::Data => "data",
                    },
                    line: error.line(),
                    column: error.column(),
                    detail: error.to_string(),
                });
            }),
        }
        .into_response()
    }
}
