#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccessKeys {
    /// The name of the key used for publishing new releases of [`cs2kz-metamod`] via GitHub
    /// Actions.
    ///
    /// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
    #[serde(default = "default_cs2kz_metamod_release_key")]
    pub cs2kz_metamod_release_key: String,
}

impl Default for AccessKeys {
    fn default() -> Self {
        Self {
            cs2kz_metamod_release_key: default_cs2kz_metamod_release_key(),
        }
    }
}

fn default_cs2kz_metamod_release_key() -> String {
    String::from("github:cs2kz-metamod:release")
}
