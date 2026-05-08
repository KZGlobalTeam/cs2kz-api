use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ReplayStorageConfig {
    pub bucket_name: String,
    pub account_id: String,
    pub access_key_id: String,
    pub access_key_secret: String,
}
