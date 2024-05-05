//! Helpers and extension traits for [`sqlx`].

mod error;

#[doc(inline)]
pub use error::SqlErrorExt;

pub mod query;

#[doc(inline)]
pub use query::{FilteredQuery, QueryBuilderExt, UpdateQuery};

mod fetch_id;

#[doc(inline)]
pub use fetch_id::FetchID;
