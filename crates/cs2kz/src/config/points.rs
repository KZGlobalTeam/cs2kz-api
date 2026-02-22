use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct PointsConfig {
    pub calc_filter_path: Option<PathBuf>,
    pub calc_run_path: Option<PathBuf>,
}
