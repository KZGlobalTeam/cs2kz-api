//! Utility trait for fetching IDs of various things from the database.

use cs2kz::{MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use futures::Future;
use sqlx::MySqlExecutor;

use crate::{Error, Result};

/// Helper trait for querying IDs from the database.
///
/// This is mainly intended for the `*Identifier` types from the [`cs2kz`] crate.
pub trait FetchID {
	/// The ID that should be fetched.
	type ID;

	/// Fetches an ID from the database if necessary.
	fn fetch_id<'c>(
		&self,
		executor: impl MySqlExecutor<'c>,
	) -> impl Future<Output = Result<Self::ID>> + Send;
}

impl FetchID for PlayerIdentifier {
	type ID = SteamID;

	async fn fetch_id<'c>(&self, executor: impl MySqlExecutor<'c>) -> Result<SteamID> {
		match self {
			Self::SteamID(steam_id) => Ok(*steam_id),
			Self::Name(name) => sqlx::query! {
				r#"
				SELECT
				  id `steam_id: SteamID`
				FROM
				  Players
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.map(|row| row.steam_id)
			.ok_or_else(|| Error::no_content()),
		}
	}
}

impl FetchID for MapIdentifier {
	type ID = u16;

	async fn fetch_id<'c>(&self, executor: impl MySqlExecutor<'c>) -> Result<u16> {
		match self {
			Self::ID(id) => Ok(*id),
			Self::Name(name) => sqlx::query! {
				r#"
				SELECT
				  id
				FROM
				  Maps
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.map(|row| row.id)
			.ok_or_else(|| Error::no_content()),
		}
	}
}

impl FetchID for ServerIdentifier {
	type ID = u16;

	async fn fetch_id<'c>(&self, executor: impl MySqlExecutor<'c>) -> Result<u16> {
		match self {
			Self::ID(id) => Ok(*id),
			Self::Name(name) => sqlx::query! {
				r#"
				SELECT
				  id
				FROM
				  Servers
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.map(|row| row.id)
			.ok_or_else(|| Error::no_content()),
		}
	}
}
