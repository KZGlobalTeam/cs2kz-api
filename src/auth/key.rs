//! Opaque API keys.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display};
use tracing::info;
use uuid::Uuid;

use crate::{Error, Result, State};

/// An opaque API key.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[debug("*****")]
#[display("{_0}")]
#[sqlx(transparent)]
pub struct Key(Uuid);

#[async_trait]
impl FromRequestParts<&'static State> for Key {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let key = header
			.token()
			.parse::<Uuid>()
			.map_err(|err| Error::key_must_be_uuid(err))?;

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
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::key_invalid())?;

		if credentials.is_expired {
			return Err(Error::key_expired());
		}

		info!(%key, "validated API key");

		Ok(Self(key))
	}
}
