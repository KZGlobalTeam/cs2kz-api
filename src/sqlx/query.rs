//! Utilities for SQL queries.

use derive_more::{Deref, DerefMut};
use sqlx::encode::IsNull;
use sqlx::{MySql, QueryBuilder};

use crate::parameters::{Limit, Offset};

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
#[derive(Deref, DerefMut)]
pub struct FilteredQuery<'q> {
	/// The underlying query builder.
	#[deref]
	#[deref_mut]
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

/// Query builder for building `UPDATE` queries.
///
/// This can be used transparently like a [`QueryBuilder`], but also has extra methods.
/// See [`UpdateQuery::set()`] for more details.
#[derive(Deref, DerefMut)]
pub struct UpdateQuery<'q> {
	/// The underlying query builder.
	#[deref]
	#[deref_mut]
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
	/// Creates a new [`UpdateQuery`] for updating the given `table`.
	///
	/// This is a wrapper over [`QueryBuilder::new()`] with a base query of `UPDATE {table}`.
	pub fn new(table: impl AsRef<str>) -> Self {
		let mut query = QueryBuilder::new("UPDATE ");
		query.push(table.as_ref()).push(' ');

		Self { query, delimiter: UpdateDelimiter::default() }
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