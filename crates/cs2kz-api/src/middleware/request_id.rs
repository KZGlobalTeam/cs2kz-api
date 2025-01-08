use tower_http::request_id::{MakeRequestId, RequestId};
use ulid::Ulid;

/// Returns an implementation of [`MakeRequestId`] that can be used with [`SetRequestIdLayer`].
///
/// [`SetRequestIdLayer`]: tower_http::request_id::SetRequestIdLayer
pub fn make_request_id() -> impl MakeRequestId + Clone {
    MakeUlidRequestId
}

/// An implementation of [`MakeRequestId`] that generates [ULID]s.
///
/// [ULID]: ulid::Ulid
#[derive(Debug, Clone, Copy)]
struct MakeUlidRequestId;

impl MakeRequestId for MakeUlidRequestId {
    #[tracing::instrument(level = "trace", skip_all, ret)]
    fn make_request_id<B>(&mut self, _: &http::Request<B>) -> Option<RequestId> {
        let ulid = Ulid::new();

        ulid.array_to_str(&mut [0; 26])
            .parse::<http::HeaderValue>()
            .map(RequestId::from)
            .ok()
    }
}
