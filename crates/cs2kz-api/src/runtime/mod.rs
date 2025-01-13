use std::io;

use tokio::runtime::Builder;
pub use tokio::runtime::Runtime;

use crate::config::RuntimeConfig;

mod environment;
pub use environment::{Environment, current as environment};

pub mod signal;

/// Builds a [Tokio runtime] according to the given `config`.
///
/// [Tokio runtime]: Config
pub fn build(config: &RuntimeConfig) -> io::Result<Runtime> {
    let mut builder = Builder::new_multi_thread();
    builder.enable_all();

    if let Some(n) = config.worker_threads {
        builder.worker_threads(n.get());
    }

    if let Some(n) = config.max_blocking_threads {
        builder.max_blocking_threads(n.get());
    }

    builder.build()
}
