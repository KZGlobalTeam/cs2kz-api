//! Helpers for building SQL queries.

use std::fmt::Display;

use derive_more::{Debug, Deref, DerefMut};
use sqlx::{MySql, QueryBuilder, Transaction};

use crate::openapi::parameters::{Limit, Offset, SortingOrder};
use crate::Result;

/// Returns the amount of **total** rows a query _could have_ returned, ignoring `LIMIT`.
///
/// NOTE: this only works if the query included `SQL_CALC_FOUND_ROWS`
pub async fn total_rows(transaction: &mut Transaction<'_, MySql>) -> Result<u64> {
	let total = sqlx::query_scalar!("SELECT FOUND_ROWS() as total")
		.fetch_one(transaction.as_mut())
		.await?
		.try_into()
		.expect("how can a count be negative");

	Ok(total)
}

/// Extension trait for [`sqlx::QueryBuilder`].
pub trait QueryBuilderExt {
	/// Pushes `LIMIT` and `OFFSET` clauses into the query.
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self;

	/// Pushes an `ORDER BY` query into the query.
	fn order_by(&mut self, order: SortingOrder, columns: impl Display) -> &mut Self;
}

impl QueryBuilderExt for QueryBuilder<'_, MySql> {
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self {
		self.push(" LIMIT ")
			.push_bind(*limit)
			.push(" OFFSET ")
			.push_bind(*offset)
	}

	fn order_by(&mut self, order: SortingOrder, columns: impl Display) -> &mut Self {
		self.push(" ORDER BY ").push(columns).push(order.sql())
	}
}

/// A query with `WHERE` / `AND` clauses.
///
/// This is a simple wrapper around [`sqlx::QueryBuilder`] that provides a [`filter()`] method to
/// push either `WHERE` or `AND` clauses into the query, depending on whether a `WHERE` clause has
/// already been pushed.
///
/// [`filter()`]: FilteredQuery::filter
#[derive(Debug, Deref, DerefMut)]
pub struct FilteredQuery<'q> {
	/// The underlying query.
	#[deref]
	#[deref_mut]
	#[debug(skip)]
	query: QueryBuilder<'q, MySql>,

	/// The current filter state.
	filter: Filter,
}

/// Query filter state.
#[derive(Debug, Default, Clone, Copy)]
enum Filter {
	/// SQL `WHERE` clause.
	#[default]
	Where,

	/// SQL `AND` clause.
	And,
}

impl Filter {
	/// Returns the corresponding SQL keyword for the current state.
	const fn sql(&self) -> &'static str {
		match self {
			Self::Where => " WHERE ",
			Self::And => " AND ",
		}
	}
}

impl<'q> FilteredQuery<'q> {
	/// Creates a new [`FilteredQuery`].
	pub fn new<S>(query: S) -> Self
	where
		S: Into<String>,
	{
		Self {
			query: QueryBuilder::new(query),
			filter: Filter::default(),
		}
	}

	/// Pushes a `WHERE` / `AND` clause into the query to filter by `column` and `value`.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = FilteredQuery::new("SELECT * FROM table");
	///
	/// if condition1 {
	///     query.filter("foo = ", bar);
	/// }
	///
	/// if condition2 {
	///     query.filter("baz > ", 69);
	/// }
	///
	/// let result = query.build().fetch_all(&database).await?;
	/// ```
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

	/// Pushes a `WHERE` / `AND` clause into the query, checking if a column is (not) `NULL`.
	pub fn filter_is_null(&mut self, column: &str, is_null: bool) -> &mut Self {
		self.query
			.push(self.filter.sql())
			.push(column)
			.push(if is_null {
				" IS NULL "
			} else {
				" IS NOT NULL "
			});

		self.filter = Filter::And;
		self
	}
}

/// An `UPDATE` query.
///
/// This is a simple wrapper around [`sqlx::QueryBuilder`] that provides a [`set()`] method to push
/// either `SET x = y` or `, x = y` into the query, depending on whether the initial `SET` clause
/// has already been pushed.
///
/// [`set()`]: UpdateQuery::set
#[derive(Debug, Deref, DerefMut)]
pub struct UpdateQuery<'q> {
	/// The underlying query.
	#[deref]
	#[deref_mut]
	#[debug(skip)]
	query: QueryBuilder<'q, MySql>,

	/// The current delimiter state.
	delimiter: UpdateDelimiter,
}

/// `UPDATE` query delimiter state.
#[derive(Debug, Default, Clone, Copy)]
enum UpdateDelimiter {
	/// SQL `SET` clause,
	#[default]
	Set,

	/// A `,`.
	Comma,
}

impl UpdateDelimiter {
	/// Returns the corresponding SQL keyword for the current state.
	const fn sql(&self) -> &'static str {
		match self {
			Self::Set => " SET ",
			Self::Comma => " , ",
		}
	}
}

impl<'q> UpdateQuery<'q> {
	/// Creates a new [`UpdateQuery`] for the given `table`.
	pub fn new<S>(table: S) -> Self
	where
		S: AsRef<str>,
	{
		let mut query = QueryBuilder::new("UPDATE ");
		query.push(table.as_ref()).push(' ');

		Self {
			query,
			delimiter: UpdateDelimiter::default(),
		}
	}

	/// Pushes a `SET x = y` / `, x = y` clause into the query.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = UpdateQuery::new("table");
	///
	/// if condition1 {
	///     query.set("foo", bar);
	/// }
	///
	/// if condition2 {
	///     query.set("baz", 69);
	/// }
	///
	/// let result = query.build().execute(&database).await?;
	/// ```
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
