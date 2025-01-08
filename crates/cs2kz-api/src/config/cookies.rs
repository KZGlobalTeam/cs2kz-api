use std::borrow::Cow;

use cookie::{Cookie, CookieBuilder, SameSite};
use cs2kz::time::DurationExt;

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct CookieConfig {
    /// The default value for the [`Domain`] field.
    ///
    /// [`Domain`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
    #[serde(default = "default_domain")]
    pub domain: String,

    /// The default value for the [`Max-Age`] field (in seconds).
    ///
    /// [`Max-Age`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#max-agenumber
    #[serde(default = "default_max_age", deserialize_with = "deserialize_max_age")]
    pub max_age: time::Duration,

    /// Same as the [`max_age`] field, but for authentication cookies.
    ///
    /// [`max_age`]: Cookies::max_age
    #[serde(default = "default_max_age_auth", deserialize_with = "deserialize_max_age")]
    pub max_age_auth: time::Duration,
}

impl CookieConfig {
    pub fn build_cookie<'a, const IS_AUTH: bool>(
        &self,
        name: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> CookieBuilder<'a> {
        Cookie::build((name, value))
            .domain(self.domain.clone())
            .http_only(IS_AUTH)
            .max_age(if IS_AUTH { self.max_age_auth } else { self.max_age })
            .path("/")
            .same_site(if IS_AUTH { SameSite::Strict } else { SameSite::Lax })
            .secure(cfg!(feature = "production"))
    }
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            domain: default_domain(),
            max_age: default_max_age(),
            max_age_auth: default_max_age_auth(),
        }
    }
}

fn default_domain() -> String {
    String::from(".cs2kz.org")
}

fn default_max_age() -> time::Duration {
    time::Duration::month() * 3
}

fn default_max_age_auth() -> time::Duration {
    time::Duration::week()
}

fn deserialize_max_age<'de, D>(deserializer: D) -> Result<time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    <i64 as serde::Deserialize<'de>>::deserialize(deserializer).map(time::Duration::seconds)
}
