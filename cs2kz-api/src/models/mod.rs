//! This module holds shared types used for database queries and HTTP responses.

pub mod players;
pub use players::Player;

pub mod maps;
pub use maps::{Course, CourseWithFilter, Filter, KZMap, Mapper};

pub mod servers;
pub use servers::{ServerResponse, ServerSummary};

pub mod jumpstats;
pub use jumpstats::JumpstatResponse;

pub mod records;
pub use records::{BhopStats, Record};

pub mod bans;
pub use bans::Ban;
