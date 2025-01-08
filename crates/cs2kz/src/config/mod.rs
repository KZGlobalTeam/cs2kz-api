mod database;
pub use database::DatabaseConfig;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
}
