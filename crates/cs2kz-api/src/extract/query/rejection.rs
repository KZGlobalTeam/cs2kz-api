use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::response::ErrorResponse;
use crate::runtime;

#[derive(Error)]
pub struct QueryRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    source: serde_html_form::de::Error,
    _marker: PhantomData<T>,
}

impl<T> QueryRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    pub(super) fn new(source: serde_html_form::de::Error) -> Self {
        Self { source, _marker: PhantomData }
    }
}

impl<T> fmt::Debug for QueryRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("QueryRejection")
            .field(&self.source)
            .finish()
    }
}

impl<T> fmt::Display for QueryRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "failed to deserialize query string")?;

        if !runtime::environment().is_production() {
            write!(fmt, " of type `{}`", type_name::<T>())?;
        }

        write!(fmt, ": {}", self.source)
    }
}

impl<T> IntoResponse for QueryRejection<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn into_response(self) -> Response {
        ErrorResponse::invalid_query_string(|details| details.set_detail(self.source.to_string()))
            .into_response()
    }
}
