//! This module contains wrappers around [`axum`]'s extractors, customizing
//! error responses.

// This module implements the replacement wrappers.
#![allow(clippy::disallowed_types)]

mod path;
pub use path::{Path, PathRejection};

mod query;
pub use query::{Query, QueryRejection};

mod json;
pub use json::{Json, JsonRejection};
