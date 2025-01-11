use clap::{Parser, Subcommand};
use cs2kz::maps::CourseFilterId;

pub fn args() -> Args {
    Args::parse()
}

#[derive(Debug, Parser)]
pub struct Args {
    /// How many rows to create.
    #[arg(short, long)]
    pub count: u64,

    /// The resource to generate.
    #[command(subcommand)]
    pub resource: Resource,
}

#[derive(Debug, Subcommand)]
pub enum Resource {
    Records {
        #[arg(long)]
        clear: bool,
        filter_id: CourseFilterId,
    },
}
