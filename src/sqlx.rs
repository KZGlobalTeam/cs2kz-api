use sqlx::error::ErrorKind;

/// Utility trait for extending [`sqlx::Error`].
pub trait SqlErrorExt {
	/// Tests whether the error is of a certain kind.
	fn is(&self, other: ErrorKind) -> bool;

	/// Tests whether the error is an FK violation.
	fn is_foreign_key_violation(&self) -> bool;

	/// Tests whether the error is a specific FK violation.
	fn is_foreign_key_violation_of(&self, fk: &str) -> bool;
}

impl SqlErrorExt for sqlx::Error {
	fn is(&self, other: ErrorKind) -> bool {
		matches!(self, Self::Database(err) if err.kind() == other)
	}

	fn is_foreign_key_violation(&self) -> bool {
		self.as_database_error()
			.map(|err| err.is_foreign_key_violation())
			.unwrap_or_default()
	}

	fn is_foreign_key_violation_of(&self, fk: &str) -> bool {
		let Some(err) = self.as_database_error() else {
			return false;
		};

		if !err.is_foreign_key_violation() {
			return false;
		}

		err.message().contains(fk)
	}
}

/// Convenience macro for decoding `NonZeroU*` types from database queries.
///
/// This can be removed once [#1926] is resolved.
///
/// [#1926]: https://github.com/launchbadge/sqlx/issues/1926
macro_rules! non_zero {
	($name:literal as $non_zero:ident, $row:expr) => {
		$row.try_get($name).and_then(|value: $non_zero| {
			TryFrom::try_from(value).map_err(|err| sqlx::Error::ColumnDecode {
				index: String::from($name),
				source: Box::new(err),
			})
		})
	};
}

pub(crate) use non_zero;
