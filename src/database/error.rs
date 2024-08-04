//! This module contains the [`SqlErrorExt`] trait.

/// Extension trait for [`sqlx::Error`].
///
/// This makes it easier to check for common error conditions that are
/// recoverable.
#[sealed]
pub trait SqlErrorExt
{
	/// Checks if the error is a "duplicate entry" error.
	fn is_duplicate_entry(&self) -> bool;

	/// Checks if the error is a foreign key violation of a specific key.
	fn is_fk_violation(&self, fk: &str) -> bool;
}

#[sealed]
impl SqlErrorExt for sqlx::Error
{
	fn is_duplicate_entry(&self) -> bool
	{
		self.as_database_error()
			.is_some_and(|e| e.code().as_deref() == Some("23000"))
	}

	fn is_fk_violation(&self, fk: &str) -> bool
	{
		self.as_database_error()
			.is_some_and(|e| e.is_foreign_key_violation() && e.message().contains(fk))
	}
}
