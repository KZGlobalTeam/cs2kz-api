//! This module contains general database utilities.
//!
//! Most notably, it exports extension traits like [`SqlErrorExt`] and
//! [`TransactionExt`] which add extra methods to [`sqlx`] types.

mod error;
pub use error::SqlErrorExt;

mod transaction;
pub use transaction::TransactionExt;

/// The minimum number of database pool connections to use.
#[cfg(test)]
pub fn min_connections() -> u32
{
	1
}

/// The minimum number of database pool connections to use.
#[cfg(not(test))]
pub fn min_connections() -> u32
{
	let available = std::thread::available_parallelism()
		.expect("system does not have parallelism?")
		.get();

	(available / 2) as u32
}

/// The maximum number of database pool connections to use.
#[cfg(test)]
pub fn max_connections() -> u32
{
	4
}

/// The maximum number of database pool connections to use.
#[cfg(not(test))]
pub fn max_connections() -> u32
{
	let available = std::thread::available_parallelism()
		.expect("system does not have parallelism?")
		.get();

	(available * 2) as u32
}
