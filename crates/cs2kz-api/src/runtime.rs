use std::sync::LazyLock;
use std::{env, io};

use tokio::runtime::Builder;
pub use tokio::runtime::Runtime;

use crate::config::RuntimeConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Local,
    Staging,
    Production,
}

impl Environment {
    pub fn is_local(&self) -> bool {
        matches!(self, Environment::Local)
    }

    pub fn is_staging(&self) -> bool {
        matches!(self, Environment::Staging)
    }

    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

pub fn environment() -> Environment {
    static ENV: LazyLock<Environment> =
        LazyLock::new(|| match env::var("KZ_API_ENVIRONMENT").map(|env| env.to_lowercase()) {
            Ok(env) => match env.as_str() {
                "local" => Environment::Local,
                "staging" => Environment::Staging,
                "production" => Environment::Production,
                value => {
                    warn!(value, "invalid `KZ_API_ENVIRONMENT`, using 'local'");
                    Environment::Local
                },
            },
            Err(env::VarError::NotPresent) => Environment::Local,
            Err(env::VarError::NotUnicode(raw)) => {
                warn!(?raw, "`KZ_API_ENVIRONMENT` is not a UTF-8 string, using 'local'");
                Environment::Local
            },
        });

    *ENV
}

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
