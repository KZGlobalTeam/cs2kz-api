//! This module holds utility types and functions for SQL queries.

use std::{cmp, fmt};

use cs2kz::{MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use sqlx::{MySql, MySqlExecutor, QueryBuilder};

use crate::{Error, Result};

/// A filter to be pushed to a [`QueryBuilder`].
///
/// Anytime you wish to push a filter parameter to the query, you push an instance of [`Filter`]
/// first, then push your parameters, and then call [`.switch()`].
///
/// [`QueryBuilder`]: sqlx::QueryBuilder
/// [`.switch()`]: Filter::switch
#[derive(Debug, Clone, Copy)]
pub enum Filter {
	/// Pushes `" WHERE "` into a query.
	Where,

	/// Pushes `" AND "` into a query.
	And,
}

impl Filter {
	/// Constructs a [`WHERE`] filter.
	///
	/// [`WHERE`]: type@Filter::Where
	pub const fn new() -> Self {
		Self::Where
	}

	/// Switches the filter to [`AND`].
	///
	/// [`AND`]: type@Filter::And
	pub fn switch(&mut self) {
		*self = Self::And;
	}
}

impl fmt::Display for Filter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Filter::Where => " WHERE ",
			Filter::And => " AND ",
		})
	}
}

/// Puhses `LIMIT` and `OFFSET` parameters to a query.
pub fn push_limits<const LIMIT: u64>(
	limit: Option<u64>,
	offset: Option<i64>,
	query: &mut QueryBuilder<'_, MySql>,
) {
	let limit = limit.map_or(100, |limit| cmp::min(limit, LIMIT));
	let offset = offset.unwrap_or_default();

	query
		.push(" LIMIT ")
		.push_bind(limit)
		.push(" OFFSET ")
		.push_bind(offset);
}

/// Fetches a [`SteamID`] for the given `player`.
pub async fn fetch_steam_id(
	player: &PlayerIdentifier<'_>,
	connection: impl MySqlExecutor<'_>,
) -> Result<SteamID> {
	match *player {
		PlayerIdentifier::SteamID(steam_id) => Ok(steam_id),
		PlayerIdentifier::Name(ref name) => {
			sqlx::query!("SELECT steam_id FROM Players WHERE name LIKE ?", format!("%{name}%"))
				.fetch_optional(connection)
				.await?
				.ok_or(Error::NoContent)?
				.steam_id
				.try_into()
				.map_err(|err| Error::Unexpected(Box::new(err)))
		}
	}
}

/// Fetches a Server ID for the given `server`.
pub async fn fetch_server_id(
	server: &ServerIdentifier<'_>,
	connection: impl MySqlExecutor<'_>,
) -> Result<u16> {
	match *server {
		ServerIdentifier::ID(id) => Ok(id),
		ServerIdentifier::Name(ref name) => {
			sqlx::query!("SELECT id FROM Servers WHERE name LIKE ?", format!("%{name}%"))
				.fetch_optional(connection)
				.await?
				.ok_or(Error::NoContent)
				.map(|row| row.id)
		}
	}
}

/// Fetches a Map ID for the given `map`.
pub async fn fetch_map_id(
	map: &MapIdentifier<'_>,
	connection: impl MySqlExecutor<'_>,
) -> Result<u16> {
	match *map {
		MapIdentifier::ID(id) => Ok(id),
		MapIdentifier::Name(ref name) => {
			sqlx::query!("SELECT id FROM Maps WHERE name LIKE ?", format!("%{name}%"))
				.fetch_optional(connection)
				.await?
				.ok_or(Error::NoContent)
				.map(|row| row.id)
		}
	}
}
