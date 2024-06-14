//! Helpers for dealing with SQL errors.

/// Extension trait for [`sqlx::Error`].
pub trait SqlErrorExt {
	/// Checks if the error is a "duplicate entry" error.
	fn is_duplicate_entry(&self) -> bool;

	/// Checks if the error is a foreign key violation of the given `fk`.
	fn is_fk_violation_of(&self, fk: &str) -> bool;
}

impl SqlErrorExt for sqlx::Error {
	fn is_duplicate_entry(&self) -> bool {
		self.as_database_error()
			.is_some_and(|err| matches!(err.code().as_deref(), Some("23000")))
	}

	fn is_fk_violation_of(&self, fk: &str) -> bool {
		self.as_database_error()
			.is_some_and(|err| err.is_foreign_key_violation() && err.message().contains(fk))
	}
}
