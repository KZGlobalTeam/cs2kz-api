//! This module contains general database utilities.
//!
//! Most notably, it exports extension traits like [`SqlErrorExt`] and
//! [`TransactionExt`] which add extra methods to [`sqlx`] types.

use std::num::NonZero;
use std::thread;

use sqlx::pool::PoolOptions;
use sqlx::{MySql, Pool};

use crate::runtime::config::DatabaseConfig;

mod error;
pub use error::SqlErrorExt;

mod transaction;
pub use transaction::TransactionExt;

/// Creates a database connection pool and runs migrations.
pub async fn create_pool(config: &DatabaseConfig) -> sqlx::Result<Pool<MySql>>
{
	let max_connections = config
		.max_connections
		.map_or_else(max_connections, NonZero::get);

	let pool = PoolOptions::new()
		.min_connections(config.min_connections)
		.max_connections(max_connections)
		.connect(config.url.as_str())
		.await?;

	sqlx::migrate!("./database/migrations").run(&pool).await?;

	Ok(pool)
}

/// The maximum number of database pool connections to use.
fn max_connections() -> u32
{
	let available = thread::available_parallelism()
		.expect("system does not support parallelism?")
		.get();

	(available * 2) as u32
}
