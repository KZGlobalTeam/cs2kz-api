use url::Url;

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct SteamAuthConfig {
    #[debug("{:?}", public_url.as_str())]
    #[serde(default = "default_public_url")]
    pub public_url: Url,

    #[debug("{:?}", redirect_to_after_login.as_str())]
    #[serde(default = "default_redirect_to_after_login")]
    pub redirect_to_after_login: Url,

    pub web_api_key: String,
}

impl Default for SteamAuthConfig {
    fn default() -> Self {
        Self {
            public_url: default_public_url(),
            redirect_to_after_login: default_redirect_to_after_login(),
            web_api_key: Default::default(),
        }
    }
}

fn default_public_url() -> Url {
    Url::parse("https://api.cs2kz.org").expect("hard-coded URL should be valid")
}

fn default_redirect_to_after_login() -> Url {
    Url::parse("https://cs2kz.org").expect("hard-coded URL should be valid")
}
