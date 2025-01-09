use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::response::ErrorResponse;
use crate::runtime;

#[derive(Error)]
pub struct PathRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    source: axum::extract::rejection::PathRejection,
    _marker: PhantomData<T>,
}

impl<T> PathRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    pub(super) fn new(source: axum::extract::rejection::PathRejection) -> Self {
        Self { source, _marker: PhantomData }
    }
}

impl<T> fmt::Debug for PathRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("PathRejection")
            .field(&self.source)
            .finish()
    }
}

impl<T> fmt::Display for PathRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "failed to deserialize path parameter(s)")?;

        if !runtime::environment().is_production() {
            write!(fmt, " of type `{}`", type_name::<T>())?;
        }

        write!(fmt, ": {}", self.source)
    }
}

impl<T> IntoResponse for PathRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn into_response(self) -> Response {
        use axum::extract::path::ErrorKind as E;

        let error = match self.source {
            axum::extract::rejection::PathRejection::FailedToDeserializePathParams(error) => error,
            error => return ErrorResponse::internal_server_error(error).into_response(),
        };

        if matches!(error.kind(), E::UnsupportedType { .. }) {
            return ErrorResponse::internal_server_error(error).into_response();
        }

        ErrorResponse::invalid_path_params(|details| match *error.kind() {
            E::WrongNumberOfParameters { got, expected } => {
                details.set_detail(format!(
                    "received wrong number of parameters (expected {expected} but got {got})"
                ));
            },
            E::ParseErrorAtKey { ref key, ref value, expected_type } => {
                details.set_detail(format!(
                    "failed to parse `{key}`: `{value}` is not a valid `{expected_type}`"
                ));
            },
            E::ParseErrorAtIndex { index, ref value, expected_type } => {
                details.set_detail(format!(
                    "failed to parse parameter at index {index}: `{value}` is not a valid `{expected_type}`"
                ));
            },
            E::ParseError { ref value, expected_type } => {
                details.set_detail(format!("failed to parse `{value}` as a `{expected_type}`"));
            },
            E::InvalidUtf8InPathParam { ref key } => {
                details.set_detail(format!("value of parameter `{key}` is not valid UTF-8"));
            },
            E::UnsupportedType { name } => {
                unreachable!("failed to deserialize `{name}` as a path parameter")
            },
            E::DeserializeError { ref key, value: _, ref message } => details
                .set_detail(format!("failed to parse value for parameter `{key}`: {message}")),
            E::Message(ref message) => details.set_detail(message.to_owned()),
            _ => {},
        })
        .into_response()
    }
}
