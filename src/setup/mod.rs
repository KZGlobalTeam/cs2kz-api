//! This module contains code relevant to the API's initial setup.
//!
//! Notably [`Error`], which is the error type returned by [`server()`].
//!
//! [`server()`]: crate::server

mod error;
pub use error::Error;
