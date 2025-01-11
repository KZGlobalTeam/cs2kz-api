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

    /// Delete the resource instead of creating it.
    #[arg(long)]
    pub clear: bool,

    /// The resource to generate.
    #[command(subcommand)]
    pub resource: Resource,
}

#[derive(Debug, Subcommand)]
pub enum Resource {
    Players,
    Servers,
    Records { filter_id: CourseFilterId },
}
