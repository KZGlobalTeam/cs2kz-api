use std::env;
use std::num::NonZero;

use serde::{Deserialize, Deserializer};
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DatabaseConfig {
    #[debug("{:?}", url.as_str())]
    #[serde(default = "default_url", deserialize_with = "deserialize_url")]
    pub url: Url,

    #[serde(default)]
    pub min_connections: u32,

    #[serde(default, deserialize_with = "deserialize_max_connections")]
    pub max_connections: Option<NonZero<u32>>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_url(),
            min_connections: 0,
            max_connections: None,
        }
    }
}

fn default_url() -> Url {
    if let Ok(url) = env::var("DATABASE_URL") {
        match url.parse::<Url>() {
            Ok(url) => return url,
            Err(error) => warn!(%error, "`DATABASE_URL` is set but is not a valid URL"),
        }
    }

    include_str!("../../../../.example.env")
        .lines()
        .find_map(|line| line.strip_prefix("DATABASE_URL="))
        .expect("example .env file should contain a default `DATABASE_URL`")
        .parse::<Url>()
        .expect("example .env file should contain a valid default `DATABASE_URL`")
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let url = Option::<Url>::deserialize(deserializer)?;
    let from_env = env::var("DATABASE_URL").map(|var| var.parse::<Url>());

    match (from_env, url) {
        (Err(env::VarError::NotPresent), fallback) => Ok(fallback.unwrap_or_else(default_url)),
        (Err(env::VarError::NotUnicode(actual)), fallback) => {
            warn!(
                actual = format_args!("`{actual:?}`"),
                "`DATABASE_URL` is set but is not valid UTF-8; falling back to config value"
            );
            Ok(fallback.unwrap_or_else(default_url))
        },
        (Ok(Err(error)), fallback) => {
            warn!(%error, "`DATABASE_URL` is set but is not a valid URL; falling back to config value");
            Ok(fallback.unwrap_or_else(default_url))
        },
        (Ok(Ok(url)), _) => Ok(url),
    }
}

fn deserialize_max_connections<'de, D>(deserializer: D) -> Result<Option<NonZero<u32>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<u32>::deserialize(deserializer).map(|value| value.and_then(NonZero::new))
}
