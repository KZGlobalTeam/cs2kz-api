//! This module contains extensions for [`sqlx::Transaction`].

use std::future::Future;

use sqlx::{MySql, Transaction};

/// Extension trait for [`sqlx::Transaction`].
#[sealed]
pub trait TransactionExt
{
	/// Returns the **total** amount of rows that _could have been_ fetched by
	/// the previous `SELECT` query, ignoring `LIMIT`.
	///
	/// NOTE: **this only works if the query contained `SQL_CALC_FOUND_ROWS`**
	fn total_rows(&mut self) -> impl Future<Output = sqlx::Result<u64>> + Send;
}

#[sealed]
impl TransactionExt for Transaction<'_, MySql>
{
	#[tracing::instrument(
		level = "trace",
		target = "cs2kz_api::database",
		err(Debug, level = "debug")
	)]
	async fn total_rows(&mut self) -> sqlx::Result<u64>
	{
		let total = sqlx::query_scalar!("SELECT FOUND_ROWS() as total")
			.fetch_one(self.as_mut())
			.await?
			.try_into()
			.expect("positive count");

		Ok(total)
	}
}
