use std::path::PathBuf;

use cs2kz::steam::WorkshopId;

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct DepotDownloaderConfig {
    #[serde(default = "default_exe_path")]
    pub exe_path: PathBuf,

    #[serde(default = "default_out_dir")]
    pub out_dir: PathBuf,
}

impl DepotDownloaderConfig {
    pub fn vpk_path(&self, workshop_id: WorkshopId) -> PathBuf {
        self.out_dir.join(format!("{workshop_id}.vpk"))
    }
}

impl Default for DepotDownloaderConfig {
    fn default() -> Self {
        Self {
            exe_path: default_exe_path(),
            out_dir: default_out_dir(),
        }
    }
}

fn default_exe_path() -> PathBuf {
    PathBuf::from("DepotDownloader")
}

fn default_out_dir() -> PathBuf {
    PathBuf::from("/var/lib/cs2kz-api/workshop/")
}
