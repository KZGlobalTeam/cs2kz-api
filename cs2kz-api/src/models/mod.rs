//! This module holds shared types used for database queries and HTTP responses.

pub mod players;
pub use players::Player;

pub mod maps;
pub use maps::{Course, CourseWithFilter, CreateCourseParams, Filter, KZMap};

pub mod servers;
pub use servers::{Server, ServerSummary};

pub mod jumpstats;
pub use jumpstats::JumpstatResponse;

pub mod records;
pub use records::{BhopStats, Record};

pub mod bans;
pub use bans::Ban;
