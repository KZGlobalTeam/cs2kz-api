//! Utility trait for fetching IDs of various things from the database.

use std::future::Future;

use cs2kz::{CourseIdentifier, MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use sqlx::MySqlExecutor;

use crate::maps::{CourseID, MapID};
use crate::servers::ServerID;
use crate::{Error, Result};

/// Helper trait for querying IDs from the database.
///
/// This is mainly intended for the `*Identifier` types from the [`cs2kz`] crate.
pub trait FetchID {
	/// The ID that should be fetched.
	type ID;

	/// Fetches an ID from the database if necessary.
	#[allow(single_use_lifetimes)]
	fn fetch_id<'c>(
		&self,
		executor: impl MySqlExecutor<'c>,
	) -> impl Future<Output = Result<Self::ID>> + Send;
}

impl FetchID for PlayerIdentifier {
	type ID = SteamID;

	async fn fetch_id(&self, executor: impl MySqlExecutor<'_>) -> Result<SteamID> {
		match *self {
			Self::SteamID(steam_id) => Ok(steam_id),
			Self::Name(ref name) => sqlx::query_scalar! {
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
			.ok_or_else(|| Error::no_content()),
		}
	}
}

impl FetchID for MapIdentifier {
	type ID = MapID;

	async fn fetch_id(&self, executor: impl MySqlExecutor<'_>) -> Result<MapID> {
		match *self {
			Self::ID(id) => Ok(MapID(id)),
			Self::Name(ref name) => sqlx::query_scalar! {
				r#"
				SELECT
				  id `id: MapID`
				FROM
				  Maps
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.ok_or_else(|| Error::no_content()),
		}
	}
}

impl FetchID for CourseIdentifier {
	type ID = CourseID;

	async fn fetch_id(&self, executor: impl MySqlExecutor<'_>) -> Result<CourseID> {
		match *self {
			CourseIdentifier::ID(course_id) => Ok(CourseID(course_id)),
			CourseIdentifier::Name(ref name) => sqlx::query_scalar! {
				r#"
				SELECT
				  id `id: CourseID`
				FROM
				  Courses
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.ok_or_else(|| Error::no_content()),
		}
	}
}

impl FetchID for ServerIdentifier {
	type ID = ServerID;

	async fn fetch_id(&self, executor: impl MySqlExecutor<'_>) -> Result<ServerID> {
		match *self {
			Self::ID(id) => Ok(ServerID(id)),
			Self::Name(ref name) => sqlx::query_scalar! {
				r#"
				SELECT
				  id `id: ServerID`
				FROM
				  Servers
				WHERE
				  name LIKE ?
				"#,
				format!("%{name}%"),
			}
			.fetch_optional(executor)
			.await?
			.ok_or_else(|| Error::no_content()),
		}
	}
}
