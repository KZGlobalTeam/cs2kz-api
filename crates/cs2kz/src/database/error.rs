pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Display, Error, From)]
#[debug("{_0}")]
#[display("database error: {_0}")]
pub struct Error(sqlx::Error);

impl Error {
    pub fn decode(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(sqlx::Error::Decode(Box::new(source)))
    }

    pub fn decode_column(
        column: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self(sqlx::Error::ColumnDecode {
            index: column.into(),
            source: Box::new(source),
        })
    }

    pub fn is_unique_violation_of(&self, key: &str) -> bool {
        self.0
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation() && error.message().contains(key))
    }

    pub fn is_fk_violation_of(&self, key: &str) -> bool {
        self.0
            .as_database_error()
            .is_some_and(|error| error.is_foreign_key_violation() && error.message().contains(key))
    }
}
