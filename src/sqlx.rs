//! Helpers and extension traits for [`sqlx`].

use std::error::Error as StdError;
use std::future::Future;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::result::Result as StdResult;
use std::time::Duration;

use cs2kz::{MapIdentifier, PlayerIdentifier, ServerIdentifier, SteamID};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::{BoxDynError, ErrorKind};
use sqlx::mysql::MySqlQueryResult;
use sqlx::{MySql, MySqlExecutor, QueryBuilder};
use thiserror::Error;

use crate::parameters::{Limit, Offset};
use crate::{Error, Result};

/// Extracts the `LAST_INSERT_ID()` from a query and parses it into some `ID`.
pub fn last_insert_id<ID>(query_result: MySqlQueryResult) -> Result<ID>
where
	NonZeroU64: TryInto<ID>,
	<NonZeroU64 as TryInto<ID>>::Error: StdError + Send + Sync + 'static,
{
	NonZeroU64::new(query_result.last_insert_id())
		.ok_or_else(|| Error::internal_server_error("PKs cannot be 0"))
		.map(TryInto::try_into)?
		.map_err(|err| Error::internal_server_error("invalid PK type").with_source(err))
}

/// Extension trait for [`sqlx::QueryBuilder`].
pub trait QueryBuilderExt {
	/// Pushes `LIMIT` and `OFFSET` clauses into the query.
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self;
}

impl QueryBuilderExt for QueryBuilder<'_, MySql> {
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self {
		self.push(" LIMIT ")
			.push_bind(limit.0)
			.push(" OFFSET ")
			.push_bind(offset.0)
	}
}

/// Query builder for inserting `WHERE` and `AND` clauses into a query.
///
/// This can be used transparently like a [`QueryBuilder`], but also has extra methods.
/// See [`FilteredQuery::filter()`] for more details.
pub struct FilteredQuery<'q> {
	/// The underlying query builder.
	query: QueryBuilder<'q, MySql>,

	/// The current state of the filter.
	filter: Filter,
}

/// State machine for determining whether to insert `WHERE` or `AND` into a query.
#[derive(Debug, Default, Clone, Copy)]
enum Filter {
	/// SQL `WHERE` clause.
	#[default]
	Where,

	/// SQL `AND` clause.
	And,
}

impl Filter {
	/// The corresponding SQL for the current state.
	fn sql(&self) -> &'static str {
		match self {
			Self::Where => " WHERE ",
			Self::And => " AND ",
		}
	}
}

impl<'q> FilteredQuery<'q> {
	/// Creates a new [`FilteredQuery`] from a base `query`.
	///
	/// This is a wrapper over [`QueryBuilder::new()`].
	pub fn new(query: impl Into<String>) -> Self {
		Self { query: QueryBuilder::new(query), filter: Filter::default() }
	}

	/// Filter by a specific `column` and compare it with a `value`.
	///
	/// This will insert `WHERE {column} {value}` into the query, which means the comparison
	/// operator must be included in `column`.
	///
	/// `WHERE` / `AND` will be inserted appropriately.
	pub fn filter<V>(&mut self, column: &str, value: V) -> &mut Self
	where
		V: sqlx::Type<MySql> + sqlx::Encode<'q, MySql> + Send + 'q,
	{
		self.query
			.push(self.filter.sql())
			.push(column)
			.push_bind(value);

		self.filter = Filter::And;
		self
	}

	/// Similar to [`FilteredQuery::filter()`], but instead of comparing a column with a value,
	/// an `IS NULL` / `IS NOT NULL` check is done instead.
	pub fn filter_is_null(&mut self, column: &str, is_null: IsNull) -> &mut Self {
		self.query
			.push(self.filter.sql())
			.push(column)
			.push(match is_null {
				IsNull::Yes => " IS NULL ",
				IsNull::No => " IS NOT NULL ",
			});

		self.filter = Filter::And;
		self
	}
}

impl<'q> Deref for FilteredQuery<'q> {
	type Target = QueryBuilder<'q, MySql>;

	fn deref(&self) -> &Self::Target {
		&self.query
	}
}

impl DerefMut for FilteredQuery<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.query
	}
}

/// Query builder for building `UPDATE` queries.
///
/// This can be used transparently like a [`QueryBuilder`], but also has extra methods.
/// See [`UpdateQuery::set()`] for more details.
pub struct UpdateQuery<'q> {
	/// The underlying query builder.
	query: QueryBuilder<'q, MySql>,

	/// The current delimiter state.
	delimiter: UpdateDelimiter,
}

/// State machine for determining whether to insert `SET` or `,` into a query.
#[derive(Debug, Default, Clone, Copy)]
enum UpdateDelimiter {
	/// SQL `SET` clause.
	#[default]
	Set,

	/// A literal `,`.
	Comma,
}

impl UpdateDelimiter {
	/// The corresponding SQL for the current state.
	fn sql(&self) -> &'static str {
		match self {
			Self::Set => " SET ",
			Self::Comma => " , ",
		}
	}
}

impl<'q> UpdateQuery<'q> {
	/// Creates a new [`UpdateQuery`] from a base `query`.
	///
	/// This is a wrapper over [`QueryBuilder::new()`].
	pub fn new(query: impl Into<String>) -> Self {
		Self {
			query: QueryBuilder::new(query),
			delimiter: UpdateDelimiter::default(),
		}
	}

	/// Set a specific `column` to some `value`.
	///
	/// This will insert `SET {column} = {value}` / `, {column} = {value}` into the query.
	pub fn set<V>(&mut self, column: &str, value: V) -> &mut Self
	where
		V: sqlx::Type<MySql> + sqlx::Encode<'q, MySql> + Send + 'q,
	{
		self.query
			.push(self.delimiter.sql())
			.push(column)
			.push(" = ")
			.push_bind(value);

		self.delimiter = UpdateDelimiter::Comma;
		self
	}
}

impl<'q> Deref for UpdateQuery<'q> {
	type Target = QueryBuilder<'q, MySql>;

	fn deref(&self) -> &Self::Target {
		&self.query
	}
}

impl DerefMut for UpdateQuery<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.query
	}
}

/// Extension trait for dealing with SQL errors.
pub trait SqlErrorExt {
	/// Checks if the error is of a given [`ErrorKind`].
	fn is(&self, kind: ErrorKind) -> bool;

	/// Checks if this is a "duplicate entry" error.
	fn is_duplicate_entry(&self) -> bool;

	/// Checks if this is a foreign key violation of a specific key.
	fn is_fk_violation_of(&self, fk: &str) -> bool;
}

impl SqlErrorExt for sqlx::Error {
	fn is(&self, kind: ErrorKind) -> bool {
		self.as_database_error()
			.is_some_and(|err| err.kind() == kind)
	}

	fn is_duplicate_entry(&self) -> bool {
		self.as_database_error()
			.is_some_and(|err| matches!(err.code().as_deref(), Some("23000")))
	}

	fn is_fk_violation_of(&self, fk: &str) -> bool {
		self.as_database_error()
			.map(|err| err.is_foreign_key_violation() && err.message().contains(fk))
			.unwrap_or_default()
	}
}

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
			.ok_or(Error::no_content()),
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
			.ok_or(Error::no_content()),
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
			.ok_or(Error::no_content()),
		}
	}
}

#[derive(Debug, Error)]
#[error("non-zero integer was zero")]
#[allow(clippy::missing_docs_in_private_items)]
pub struct NonZeroIntWasZero;

/// Helper macro for parsing the `NonZero*` types from the standard library out of query results.
macro_rules! non_zero {
	($col:literal as $ty:ty, $row:expr) => {
		$row.try_get($col)
			.map(<$ty>::new)?
			.ok_or_else(|| sqlx::Error::ColumnDecode {
				index: String::from($col),
				source: Box::new($crate::sqlx::NonZeroIntWasZero),
			})
	};
}

pub(crate) use non_zero;

/// Wrapper around [`std::time::Duration`], which takes care of encoding / decoding as seconds.
#[derive(Debug)]
pub struct Seconds(Duration);

impl From<Seconds> for Duration {
	fn from(value: Seconds) -> Self {
		value.0
	}
}

impl sqlx::Type<MySql> for Seconds {
	fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
		f64::type_info()
	}
}

impl<'q> sqlx::Encode<'q, MySql> for Seconds {
	fn encode_by_ref(&self, buf: &mut <MySql as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
		self.0.as_secs_f64().encode_by_ref(buf)
	}
}

impl<'q> sqlx::Decode<'q, MySql> for Seconds {
	fn decode(value: <MySql as HasValueRef<'q>>::ValueRef) -> StdResult<Self, BoxDynError> {
		f64::decode(value).map(Duration::from_secs_f64).map(Self)
	}
}
