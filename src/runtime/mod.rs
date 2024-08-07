//! This module contains code relevant to the API's runtime, such as signal
//! handlers or [`Config`].

pub mod signals;
pub mod panic_hook;

pub mod config;
pub use config::Config;
