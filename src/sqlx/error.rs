//! Utilities for dealing with database errors.

use sqlx::error::ErrorKind;

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
