use std::env;
use std::sync::LazyLock;

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

pub fn current() -> Environment {
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
