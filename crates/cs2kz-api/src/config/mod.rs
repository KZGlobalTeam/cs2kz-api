mod server;
pub use server::ServerConfig;

pub mod tracing;
pub use tracing::TracingConfig;

mod runtime;
pub use runtime::RuntimeConfig;

mod access_keys;
pub use access_keys::AccessKeys;

mod cookies;
pub use cookies::CookieConfig;

mod steam_auth;
pub use steam_auth::SteamAuthConfig;

mod depot_downloader;
pub use depot_downloader::DepotDownloaderConfig;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    /// Configuration for the HTTP server.
    pub server: ServerConfig,

    /// Configuration for [`tracing-subscriber`].
    pub tracing: TracingConfig,

    /// Configuration for Tokio.
    pub runtime: RuntimeConfig,

    /// Names of known access keys.
    pub access_keys: AccessKeys,

    /// Default values for HTTP cookie fields.
    pub cookies: CookieConfig,

    pub steam_auth: SteamAuthConfig,
    pub depot_downloader: DepotDownloaderConfig,

    #[serde(flatten)]
    pub cs2kz: cs2kz::Config,
}
