use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct TracingConfig {
    /// Whether to initialize a subscriber at all.
    pub enable: bool,

    /// Configuration for the stderr output.
    pub stderr: StderrConfig,

    /// Configuration for the log files output.
    pub files: FilesConfig,

    /// Configuration for `tokio-console`.
    pub console: ConsoleConfig,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct StderrConfig {
    /// Whether to emit traces to stderr.
    pub enable: bool,

    /// Whether to include ANSI escape codes for colors.
    pub ansi: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct FilesConfig {
    /// Whether to emit traces to log files.
    pub enable: bool,

    /// Path to the directory where the log files should be stored.
    #[serde(default = "default_files_directory")]
    pub directory: PathBuf,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            enable: false,
            directory: default_files_directory(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct ConsoleConfig {
    /// Whether to emit traces to `tokio-console`.
    pub enable: bool,

    /// How to expose the gRPC server.
    #[serde(default = "default_console_server_addr")]
    pub server_addr: ConsoleServerAddr,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enable: false,
            server_addr: default_console_server_addr(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ConsoleServerAddr {
    /// TCP socket address.
    Tcp(SocketAddr),

    /// Path to a UDS.
    Unix(PathBuf),
}

impl From<ConsoleServerAddr> for console_subscriber::ServerAddr {
    fn from(addr: ConsoleServerAddr) -> Self {
        match addr {
            ConsoleServerAddr::Tcp(addr) => Self::Tcp(addr),
            ConsoleServerAddr::Unix(path) => Self::Unix(path),
        }
    }
}

fn default_files_directory() -> PathBuf {
    PathBuf::from("/var/log/cs2kz-api")
}

fn default_console_server_addr() -> ConsoleServerAddr {
    ConsoleServerAddr::Tcp(SocketAddr::new(
        console_subscriber::Server::DEFAULT_IP,
        console_subscriber::Server::DEFAULT_PORT,
    ))
}
