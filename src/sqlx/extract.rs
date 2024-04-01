//! Axum extractors for [`sqlx`] types.

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use derive_more::{Deref, DerefMut};
use sqlx::pool::PoolConnection;
use sqlx::{MySql, Pool};

/// Acquires a database connection when extracted and returns it to a connection pool when dropped.
#[derive(Deref, DerefMut)]
pub struct Connection(pub PoolConnection<MySql>);

#[async_trait]
impl<S> FromRequestParts<S> for Connection
where
	S: Send + Sync,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = crate::Error;

	async fn from_request_parts(
		_: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let pool = Pool::from_ref(state);
		let connection = pool.acquire().await?;

		Ok(Self(connection))
	}
}

/// Begins a transaction when extracted and aborts it if [`sqlx::Transaction::commit()`] is not
/// called.
#[derive(Deref, DerefMut)]
pub struct Transaction(pub sqlx::Transaction<'static, MySql>);

#[async_trait]
impl<S> FromRequestParts<S> for Transaction
where
	S: Send + Sync,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = crate::Error;

	async fn from_request_parts(
		_: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let pool = Pool::from_ref(state);
		let transaction = pool.begin().await?;

		Ok(Self(transaction))
	}
}
