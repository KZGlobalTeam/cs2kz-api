use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

use axum::response::{IntoResponse, Response};
use headers::Header as IsHeader;

use crate::response::ErrorResponse;

pub struct HeaderRejection<H: IsHeader> {
    reason: Reason,
    _marker: PhantomData<H>,
}

#[derive(Debug)]
enum Reason {
    Missing,
    Parse(headers::Error),
}

impl<H: IsHeader> HeaderRejection<H> {
    fn new(reason: Reason) -> Self {
        Self { reason, _marker: PhantomData }
    }

    pub(super) fn missing() -> Self {
        Self::new(Reason::Missing)
    }

    pub(super) fn parse(error: headers::Error) -> Self {
        Self::new(Reason::Parse(error))
    }
}

impl<H: IsHeader> fmt::Debug for HeaderRejection<H> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("HeaderRejection")
            .field("header", H::name())
            .field("reason", &self.reason)
            .finish()
    }
}

impl<H: IsHeader> fmt::Display for HeaderRejection<H> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reason {
            Reason::Missing => write!(fmt, "missing `{}` header", H::name()),
            Reason::Parse(_) => write!(fmt, "failed to parse `{}` header", H::name()),
        }
    }
}

impl<H: IsHeader> std::error::Error for HeaderRejection<H> {}

impl<H: IsHeader> IntoResponse for HeaderRejection<H> {
    fn into_response(self) -> Response {
        if type_name::<H>().contains("Authorization<") {
            return ErrorResponse::unauthorized().into_response();
        }

        match self.reason {
            Reason::Missing => ErrorResponse::missing_header::<H>(),
            Reason::Parse(error) => ErrorResponse::invalid_header::<H>(|details| {
                details.set_detail(error.to_string());
            }),
        }
        .into_response()
    }
}
