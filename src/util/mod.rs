//! This module contains general purpose utilities that didn't find a better
//! home yet.

#[doc(hidden)]
pub(crate) mod name_or_id;
pub use name_or_id::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier};

mod is_empty;
pub use is_empty::IsEmpty;
