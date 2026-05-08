mod database;
pub use database::DatabaseConfig;

mod points;
pub use points::PointsConfig;

mod replays;
pub use replays::ReplayStorageConfig;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub points: PointsConfig,

    #[serde(default)]
    pub replay_storage: Option<ReplayStorageConfig>,
}
