//! This module contains general purpose middleware.
//!
//! Middlewares are implemented as [tower services].
//! This means they can integrate with [`axum`], our HTTP framework, but are
//! also re-usable independently of that.
//!
//! [tower services]: tower::Service

pub(crate) mod logging;
pub(crate) mod panic_handler;
pub(crate) mod cors;

pub mod infallible;
pub use infallible::InfallibleLayer;
