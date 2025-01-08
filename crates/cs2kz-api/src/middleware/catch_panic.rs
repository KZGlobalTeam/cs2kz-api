use std::any::Any;
use std::convert::Infallible;

use axum::body::HttpBody;
use bytes::Bytes;
use http_body_util::Empty;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

/// Returns a [`tower::Layer`] for catching panics that occur while handling requests.
///
/// This is purely a safety guard! If a handler ever panics, that's a bug.
pub fn layer() -> CatchPanicLayer<
    impl ResponseForPanic<ResponseBody: HttpBody<Data = Bytes, Error = Infallible> + Send + 'static>,
> {
    CatchPanicLayer::custom(PanicResponse)
}

/// An implementation of [`ResponseForPanic`].
#[derive(Debug, Clone, Copy)]
struct PanicResponse;

impl ResponseForPanic for PanicResponse {
    type ResponseBody = Empty<Bytes>;

    fn response_for_panic(
        &mut self,
        panic_payload: Box<dyn Any + Send + 'static>,
    ) -> http::Response<Self::ResponseBody> {
        let error = panic_payload
            .downcast_ref::<&'static str>()
            .copied()
            .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<unavailable>");

        error!(%error, "request handler panicked");

        http::Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Empty::new())
            .unwrap()
    }
}
