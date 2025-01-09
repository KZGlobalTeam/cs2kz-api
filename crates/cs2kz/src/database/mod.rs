use std::num::NonZero;

use sqlx::migrate::Migrator;
use sqlx::mysql::{MySql, MySqlConnection, MySqlPoolOptions};
use url::Url;

mod macros;
pub(crate) use macros::*;

mod error;
pub use error::{Error, Result};

pub type Connection = MySqlConnection;

pub static MIGRATIONS: Migrator = sqlx::migrate!();

/// A handle to the API's database.
#[derive(Debug, AsRef, Clone)]
#[debug("{}", std::any::type_name::<Driver>())]
pub struct Database<Driver: sqlx::Database = MySql> {
    connections: sqlx::Pool<Driver>,
}

#[derive(Debug)]
pub struct DatabaseConnectionOptions<'a> {
    pub url: &'a Url,
    pub min_connections: u32,
    pub max_connections: Option<NonZero<u32>>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to establish database connection: {_0}")]
pub struct EstablishDatabaseConnectionError(sqlx::Error);

impl Database {
    #[tracing::instrument(level = "debug", fields(%url), err)]
    pub async fn connect(
        DatabaseConnectionOptions { url, min_connections, max_connections }: DatabaseConnectionOptions<'_>,
    ) -> Result<Self, EstablishDatabaseConnectionError> {
        let max_connections = max_connections.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(NonZero::<u32>::MIN, |amount| {
                amount.try_into().expect("sensible core count")
            })
        });

        MySqlPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections.get())
            .connect(url.as_str())
            .await
            .map(|pool| Self { connections: pool })
            .map_err(EstablishDatabaseConnectionError)
    }
}

impl<Driver: sqlx::Database> Database<Driver> {
    /// Executes an `async` closure in the context of a transaction.
    ///
    /// If the closure returns <code>[Ok](())</code>, the transaction will be committed.
    /// If the closure returns <code>[Err](_)</code> or panics, the transaction will be rolled
    /// back.
    pub async fn in_transaction<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: AsyncFnOnce(&mut Driver::Connection) -> Result<T, E>,
        E: From<Error>,
    {
        let mut txn = self.connections.begin().await.map_err(Error::from)?;

        match f(&mut txn).await {
            Ok(value) => {
                txn.commit().await.map_err(Error::from)?;
                Ok(value)
            },
            Err(error) => {
                txn.rollback().await.map_err(Error::from)?;
                Err(error)
            },
        }
    }

    /// Closes the connection pool.
    ///
    /// Any queries made after this call completes will fail.
    #[tracing::instrument(level = "trace")]
    pub async fn cleanup(&self) {
        self.connections.close().await;
    }
}
