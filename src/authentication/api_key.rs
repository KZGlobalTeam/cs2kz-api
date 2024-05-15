//! Everything related to API key authentication.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display, Into};
use tracing::debug;
use uuid::Uuid;

use crate::{Error, Result, State};

/// An opaque API key.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Into)]
#[debug("{name}")]
#[display("{key} ({name})")]
pub struct ApiKey {
	/// The secret key.
	#[into]
	key: Uuid,

	/// The name of the key.
	name: String,
}

impl ApiKey {
	/// Generate a new [`ApiKey`].
	pub fn new<S>(name: S) -> Self
	where
		S: Into<String>,
	{
		Self {
			key: Uuid::new_v4(),
			name: name.into(),
		}
	}

	/// Returns the name of this key.
	pub fn name(&self) -> &str {
		&self.name
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for ApiKey {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let key = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await?
			.token()
			.parse::<Uuid>()
			.map_err(Error::key_must_be_uuid)?;

		let api_key = sqlx::query! {
			r#"
			SELECT
			  name,
			  COALESCE((expires_on < NOW()), FALSE) `is_expired!: bool`
			FROM
			  Credentials
			WHERE
			  `key` = ?
			"#,
			key,
		}
		.fetch_optional(&state.database)
		.await?
		.map(|row| match row.is_expired {
			true => Err(Error::key_expired()),
			false => Ok(ApiKey {
				key,
				name: row.name,
			}),
		})
		.ok_or_else(|| Error::key_invalid())??;

		debug!(?api_key, "authenticated API key");

		Ok(api_key)
	}
}
