//! CLI argument handling.

use clap::Parser;
use url::Url;

pub fn args() -> Args {
    Args::parse()
}

#[derive(Debug, Parser)]
pub struct Args {
    /// The URL of the database the daemon should connect to.
    #[arg(long)]
    pub database_url: Option<Url>,
}
