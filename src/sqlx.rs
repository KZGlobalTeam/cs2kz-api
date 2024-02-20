use sqlx::error::ErrorKind;

pub trait SqlErrorExt {
	fn is(&self, other: ErrorKind) -> bool;
	fn is_foreign_key_violation(&self) -> bool;
	fn is_foreign_key_violation_of(&self, fk: &str) -> bool;
}

impl SqlErrorExt for sqlx::Error {
	fn is(&self, other: ErrorKind) -> bool {
		matches!(self, sqlx::Error::Database(err) if err.kind() == other)
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
