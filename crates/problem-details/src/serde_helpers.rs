use std::fmt;

use serde::{Serialize, Serializer};

pub(crate) struct SerializeUri<'a> {
    uri: &'a http::Uri,
}

impl<'a> SerializeUri<'a> {
    pub(crate) fn new(uri: &'a http::Uri) -> Self {
        Self { uri }
    }
}

impl Serialize for SerializeUri<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args!("{}", self.uri).serialize(serializer)
    }
}

pub(crate) struct SerializeStatusCode {
    status: http::StatusCode,
}

impl SerializeStatusCode {
    pub(crate) fn new(status: http::StatusCode) -> Self {
        Self { status }
    }
}

impl Serialize for SerializeStatusCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format_args
    }
}
