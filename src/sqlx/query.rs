//! Utilities for SQL queries.

use std::fmt::Display;

use derive_more::{Debug, Deref, DerefMut};
use sqlx::{MySql, QueryBuilder, Transaction};

use crate::openapi::parameters::{Limit, Offset, SortingOrder};

/// Returns the total amount of rows that _could_ have been fetched from a query containing
/// `LIMIT`. This only works for queries containing `SQL_CALC_FOUND_ROWS`.
pub async fn total_rows(transaction: &mut Transaction<'_, MySql>) -> crate::Result<u64> {
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

	/// Pushes an `ORDER BY` clause into the query.
	fn order_by(&mut self, order: SortingOrder, columns: impl Display) -> &mut Self;
}

impl QueryBuilderExt for QueryBuilder<'_, MySql> {
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self {
		self.push(" LIMIT ")
			.push_bind(limit.0)
			.push(" OFFSET ")
			.push_bind(offset.0)
	}

	fn order_by(&mut self, order: SortingOrder, columns: impl Display) -> &mut Self {
		self.push(" ORDER BY ").push(columns).push(order.sql())
	}
}

/// Query builder for inserting `WHERE` and `AND` clauses into a query.
///
/// This can be used transparently like a [`QueryBuilder`], but also has extra methods.
/// See [`FilteredQuery::filter()`] for more details.
#[derive(Debug, Deref, DerefMut)]
pub struct FilteredQuery<'q> {
	/// The underlying query builder.
	#[deref]
	#[deref_mut]
	#[debug(skip)]
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
	const fn sql(&self) -> &'static str {
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
	pub fn new<S>(query: S) -> Self
	where
		S: Into<String>,
	{
		Self {
			query: QueryBuilder::new(query),
			filter: Filter::default(),
		}
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

/// Query builder for building `UPDATE` queries.
///
/// This can be used transparently like a [`QueryBuilder`], but also has extra methods.
/// See [`UpdateQuery::set()`] for more details.
#[derive(Debug, Deref, DerefMut)]
pub struct UpdateQuery<'q> {
	/// The underlying query builder.
	#[deref]
	#[deref_mut]
	#[debug(skip)]
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
	const fn sql(&self) -> &'static str {
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
