//! This module contains wrappers around [`axum`]'s extractors, customizing
//! error responses.

#![expect(clippy::disallowed_types, reason = "this module implements the replacement wrappers")]

mod path;
pub use path::{Path, PathRejection};

mod query;
pub use query::{Query, QueryRejection};

mod json;
pub use json::{Json, JsonRejection};
