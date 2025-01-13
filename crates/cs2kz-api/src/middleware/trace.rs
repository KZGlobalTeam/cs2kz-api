use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::body::HttpBody;
use axum::extract::ConnectInfo;
use bytes::Buf;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::RequestId;
use tower_http::trace::{
    HttpMakeClassifier,
    MakeSpan,
    OnBodyChunk,
    OnEos,
    OnFailure,
    OnRequest,
    OnResponse,
    TraceLayer,
};

/// Returns a [`tower::Layer`] for emitting [`tracing`] events as requests are processed.
pub fn layer<RequestBody, ResponseBody>() -> TraceLayer<
    HttpMakeClassifier,
    impl MakeSpan<RequestBody> + Clone,
    impl OnRequest<RequestBody> + Clone,
    impl OnResponse<ResponseBody> + Clone,
    impl OnBodyChunk<ResponseBody::Data> + Clone,
    impl OnEos + Clone,
    impl OnFailure<ServerErrorsFailureClass> + Clone,
>
where
    RequestBody: HttpBody,
    ResponseBody: HttpBody<Error: fmt::Display + 'static>,
{
    TraceLayer::new_for_http()
        .make_span_with(make_span::<RequestBody>)
        .on_request(on_request::<RequestBody>)
        .on_response(on_response::<ResponseBody>)
        .on_body_chunk(on_body_chunk::<ResponseBody>)
        .on_eos(on_eos)
        .on_failure(on_failure)
}

/// Called at the start of each request cycle. This function generates the [`tracing::Span`] that
/// is passed to the other functions later on.
fn make_span<B>(request: &http::Request<B>) -> tracing::Span {
    fn extract_cf_connecting_ip(headers: &http::HeaderMap) -> Option<IpAddr> {
        headers
            .get("CF-Connecting-IP")?
            .to_str()
            .inspect_err(|err| trace!(%err, "`CF-Connecting-IP` header was not UTF-8"))
            .ok()?
            .parse::<IpAddr>()
            .inspect_err(|err| trace!(%err, "`CF-Connecting-IP` header is not an IP address"))
            .ok()
    }

    fn extract_connect_info(extensions: &http::Extensions) -> Option<IpAddr> {
        extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|&ConnectInfo(addr)| addr.ip())
    }

    let client_addr = extract_cf_connecting_ip(request.headers())
        .or_else(|| extract_connect_info(request.extensions()))
        .expect("`ConnectInfo` should be injected by the router");

    let request_id = request
        .extensions()
        .get::<RequestId>()
        .expect("`RequestId` should be injected by previous middleware");

    info_span! {
        target: "cs2kz_api::http",
        "request",
        %client_addr,
        request.id = ?request_id.header_value(),
        request.method = tracing::field::Empty,
        request.uri = tracing::field::Empty,
        request.version = tracing::field::Empty,
        request.headers = tracing::field::Empty,
        response.status = tracing::field::Empty,
        response.headers = tracing::field::Empty,
    }
}

/// Called right after [`make_span`] to signal that the request is now being processed.
fn on_request<B>(request: &http::Request<B>, span: &tracing::Span) {
    span.record("request.method", tracing::field::debug(request.method()));
    span.record("request.uri", tracing::field::debug(request.uri()));
    span.record("request.version", tracing::field::debug(request.version()));
    span.record("request.headers", tracing::field::debug(request.headers()));

    info!(target: "cs2kz_api::http::request", "starting to process request");
}

/// Called after the inner service has produced a response.
fn on_response<B>(response: &http::Response<B>, latency: Duration, span: &tracing::Span) {
    span.record("response.status", response.status().as_u16());
    span.record("response.headers", tracing::field::debug(response.headers()));

    info!(target: "cs2kz_api::http::response", ?latency, "finished processing request");
}

/// Called for every chunk of data sent by the response body.
fn on_body_chunk<B: HttpBody>(chunk: &B::Data, latency: Duration, _span: &tracing::Span) {
    trace!(target: "cs2kz_api::http::response::body::chunk", size = chunk.remaining(), ?latency);
}

/// Called after the response body has finished streaming.
fn on_eos(trailers: Option<&http::HeaderMap>, stream_duration: Duration, _span: &tracing::Span) {
    debug!(target: "cs2kz_api::http::response", ?trailers, ?stream_duration);
}

/// Called for every response that was classified to be a failure.
fn on_failure(failure_class: ServerErrorsFailureClass, latency: Duration, span: &tracing::Span) {
    match failure_class {
        ServerErrorsFailureClass::StatusCode(status) => {
            span.record("response.status", status.as_u16());
            error!(target: "cs2kz_api::http", status = status.as_u16(), ?latency, "failed to handle request");
        },
        ServerErrorsFailureClass::Error(error) => {
            error!(target: "cs2kz_api::http", %error, "failed to handle request");
        },
    }
}
