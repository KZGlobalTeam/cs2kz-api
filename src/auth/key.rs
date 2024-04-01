//! Opaque API keys.

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display};
use sqlx::{MySql, Pool};
use tracing::info;
use uuid::Uuid;

use crate::{Error, Result};

/// An opaque API key.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[debug("*****")]
#[display("{_0}")]
#[sqlx(transparent)]
pub struct Key(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for Key
where
	S: Send + Sync,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = Error;

	async fn from_request_parts(parts: &mut request::Parts, state: &S) -> Result<Self> {
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let key = header
			.token()
			.parse::<Uuid>()
			.map_err(|err| Error::key_must_be_uuid(err))?;

		let database = Pool::from_ref(state);

		let credentials = sqlx::query! {
			r#"
			SELECT
			  name,
			  COALESCE((expires_on < NOW()), FALSE) `is_expired!: bool`
			FROM
			  Credentials
			WHERE
			  token = ?
			"#,
			key,
		}
		.fetch_optional(&database)
		.await?
		.ok_or_else(|| Error::key_invalid())?;

		if credentials.is_expired {
			return Err(Error::key_expired());
		}

		info!(%key, "validated API key");

		Ok(Self(key))
	}
}
