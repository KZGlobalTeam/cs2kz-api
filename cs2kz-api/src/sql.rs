//! This module holds utility types and functions for SQL queries.

use std::future::Future;
use std::{cmp, fmt};

use cs2kz::{MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use sqlx::{Encode, MySql, MySqlExecutor, QueryBuilder, Type};

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

/// Pushes values into an `SELECT * FROM table WHERE col IN (...)` query.
///
/// # Example
///
/// ```ignore
/// let mut query = QueryBuilder::new("SELECT * FROM table WHERE col IN");
/// let values = [1, 2, 3];
///
/// push_tuple(&values, &mut query);
///
/// let sql = query.sql();
///
/// // `1, 2, 3` get bound to this query when it is executed.
/// assert_eq!(sql, "SELECT * FROM table WHERE col IN (?, ?, ?)");
/// ```
pub fn push_tuple<'query, 'args, T>(args: &'args [T], query: &mut QueryBuilder<'args, MySql>)
where
	&'args T: Encode<'args, MySql> + Type<MySql> + Send,
{
	query.push("(");

	let mut separated = query.separated(", ");

	for arg in args {
		separated.push_bind(arg);
	}

	separated.push_unseparated(")");
}

/// A convenience trait for fetching IDs from the database.
pub trait FetchID {
	/// The ID to be fetched.
	type ID;

	/// Fetches the type's ID from a database connection.
	fn fetch_id<'c>(
		&self,
		connection: impl MySqlExecutor<'c>,
	) -> impl Future<Output = Result<Self::ID>>;
}

impl FetchID for PlayerIdentifier<'_> {
	type ID = SteamID;

	async fn fetch_id<'c>(&self, connection: impl MySqlExecutor<'c>) -> Result<Self::ID> {
		match self {
			PlayerIdentifier::SteamID(steam_id) => Ok(*steam_id),
			PlayerIdentifier::Name(name) => {
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
}

impl FetchID for ServerIdentifier<'_> {
	type ID = u16;

	async fn fetch_id<'c>(&self, connection: impl MySqlExecutor<'c>) -> Result<u16> {
		match self {
			ServerIdentifier::ID(id) => Ok(*id),
			ServerIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Servers WHERE name LIKE ?", format!("%{name}%"))
					.fetch_optional(connection)
					.await?
					.ok_or(Error::NoContent)
					.map(|row| row.id)
			}
		}
	}
}

impl FetchID for MapIdentifier<'_> {
	type ID = u16;

	async fn fetch_id<'c>(&self, connection: impl MySqlExecutor<'c>) -> Result<u16> {
		match self {
			MapIdentifier::ID(id) => Ok(*id),
			MapIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Maps WHERE name LIKE ?", format!("%{name}%"))
					.fetch_optional(connection)
					.await?
					.ok_or(Error::NoContent)
					.map(|row| row.id)
			}
		}
	}
}
