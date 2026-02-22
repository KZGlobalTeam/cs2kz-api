use clap::{Parser, Subcommand};
use cs2kz::git::GitRevision;
use cs2kz::maps::{CourseFilterId, MapId};
use cs2kz::players::PlayerId;
use cs2kz::records::RecordId;
use cs2kz::servers::ServerId;
use cs2kz::users::UserId;

pub fn args() -> Args {
    Args::parse()
}

#[derive(Debug, Parser)]
pub struct Args {
    /// The resource to generate.
    #[command(subcommand)]
    pub resource: Resource,
}

#[derive(Debug, Subcommand)]
pub enum Resource {
    PluginVersions {
        #[command(subcommand)]
        action: PluginVersionAction,
    },
    Players {
        #[command(subcommand)]
        action: PlayerAction,
    },
    Servers {
        #[command(subcommand)]
        action: ServerAction,
    },
    Records {
        #[command(subcommand)]
        action: RecordAction,
    },
    Maps {
        #[command(subcommand)]
        action: MapAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum PluginVersionAction {
    Create {
        #[arg(long = "git")]
        git_revision: Option<GitRevision>,

        version: semver::Version,
    },
    Delete {
        version: semver::Version,
    },
}

#[derive(Debug, Subcommand)]
pub enum PlayerAction {
    Create { count: usize },
    Delete { count: usize },
}

#[derive(Debug, Subcommand)]
pub enum ServerAction {
    Create {
        #[arg(long = "owner")]
        owner_id: Option<UserId>,

        count: usize,
    },
    Delete {
        #[arg(long = "owner")]
        owner_id: Option<UserId>,

        count: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum RecordAction {
    Create {
        #[arg(long = "player")]
        player_id: Option<PlayerId>,

        #[arg(long = "server")]
        server_id: Option<ServerId>,

        #[arg(long = "filter")]
        filter_id: Option<CourseFilterId>,

        #[arg(long)]
        plugin_version: Option<semver::Version>,

        count: usize,
    },
    Delete {
        /// Delete records with this filter
        #[arg(long = "filter")]
        filter_id: Option<CourseFilterId>,

        /// Delete records starting at this ID
        #[arg(long)]
        starting_at: Option<RecordId>,

        count: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum MapAction {
    Create {
        #[arg(long)]
        mappers: Vec<PlayerId>,

        count: usize,
    },
    Delete {
        /// Delete maps starting at this ID
        #[arg(long)]
        starting_at: Option<MapId>,

        count: usize,
    },
}
