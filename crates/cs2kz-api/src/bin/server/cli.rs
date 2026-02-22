//! CLI argument handling.

use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

pub fn args() -> Args {
    Args::parse()
}

#[derive(Debug, Parser)]
pub struct Args {
    /// The IP address the HTTP server will listen on.
    ///
    /// This takes precedence over the value in the configuration file.
    #[arg(long = "ip")]
    pub ip_addr: Option<IpAddr>,

    /// The port the HTTP server will listen on.
    ///
    /// This takes precedence over the value in the configuration file.
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Path to the configuration file.
    ///
    /// Will default to `./cs2kz-api.toml` if unspecified.
    /// If that file does not exist, default configuration values will be used.
    #[arg(short, long = "config")]
    pub config_path: Option<PathBuf>,

    /// Path to the `DepotDownloader` executable the API should use.
    #[arg(long)]
    pub depot_downloader_path: Option<PathBuf>,

    /// Path to a directory containing the `calc_filter.py` and `calc_run.py` scripts.
    #[arg(long)]
    pub scripts_path: Option<PathBuf>,
}

impl Args {
    /// Applies any overrides specified as CLI flags to the given config.
    pub fn apply_to_config(&self, config: &mut cs2kz_api::Config) {
        if let Some(ip_addr) = self.ip_addr {
            config.server.ip_addr = ip_addr;
        }

        if let Some(port) = self.port {
            config.server.port = port;
        }

        if let Some(ref path) = self.depot_downloader_path {
            config.depot_downloader.exe_path = path.clone();
        }

        if let Some(ref path) = self.scripts_path {
            config.cs2kz.points.calc_filter_path = Some(path.join("calc_filter.py"));
            config.cs2kz.points.calc_run_path = Some(path.join("calc_run.py"));
        }
    }
}
