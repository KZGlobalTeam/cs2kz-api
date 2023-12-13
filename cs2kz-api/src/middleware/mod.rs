//! This module holds various middleware functions.

pub mod logging;
pub use logging::log_request;

pub mod auth;
