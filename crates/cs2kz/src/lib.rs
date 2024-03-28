//! This is the core library for everything [cs2kz] related written in Rust.
//!
//! Type definitions and trait implementation with various crates from the ecosystem are
//! provided by this crate, so that you don't have to write the same abstractions over and over
//! again for your own projects.
//!
//! This library is primarily used by the [API], but feel free to include it in your own
//! projects!
//!
//! [cs2kz]: https://github.com/KZGlobalTeam/cs2kz-metamod
//! [API]: https://github.com/KZGlobalTeam/cs2kz-api

#![deny(missing_docs, rustdoc::broken_intra_doc_links, missing_debug_implementations, clippy::perf)]
#![warn(clippy::cognitive_complexity, clippy::missing_const_for_fn)]

mod error;
pub use error::{Error, Result};

pub mod steam_id;
pub use steam_id::SteamID;

pub mod mode;
pub use mode::Mode;

pub mod style;
pub use style::Style;

pub mod tier;
pub use tier::Tier;

pub mod jumptype;
pub use jumptype::JumpType;

pub mod player_identifier;
pub use player_identifier::PlayerIdentifier;

pub mod map_identifier;
pub use map_identifier::MapIdentifier;

pub mod server_identifier;
pub use server_identifier::ServerIdentifier;

pub mod global_status;
pub use global_status::GlobalStatus;

pub mod ranked_status;
pub use ranked_status::RankedStatus;
