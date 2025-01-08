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
    }
}
