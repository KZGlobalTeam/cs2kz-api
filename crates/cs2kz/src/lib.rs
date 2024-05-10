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

#[doc(inline)]
pub use error::{Error, Result};

mod steam_id;

#[doc(inline)]
pub use steam_id::SteamID;

mod mode;

#[doc(inline)]
pub use mode::Mode;

mod style;

#[doc(inline)]
pub use style::Style;

mod tier;

#[doc(inline)]
pub use tier::Tier;

mod jumptype;

#[doc(inline)]
pub use jumptype::JumpType;

mod player_identifier;

#[doc(inline)]
pub use player_identifier::PlayerIdentifier;

mod map_identifier;

#[doc(inline)]
pub use map_identifier::MapIdentifier;

mod course_identifier;

#[doc(inline)]
pub use course_identifier::CourseIdentifier;

mod server_identifier;

#[doc(inline)]
pub use server_identifier::ServerIdentifier;

mod global_status;

#[doc(inline)]
pub use global_status::GlobalStatus;

mod ranked_status;

#[doc(inline)]
pub use ranked_status::RankedStatus;
