use sqlx::error::ErrorKind;

pub trait IsError {
	fn is(&self, other: ErrorKind) -> bool;
}

impl IsError for sqlx::Error {
	fn is(&self, other: ErrorKind) -> bool {
		matches!(self, sqlx::Error::Database(err) if err.kind() == other)
	}
}
