//! Everything related to API key authentication.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display, Into};
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
	#[tracing::instrument(level = "debug", name = "auth::api_key::new", skip_all, fields(
		name = tracing::field::Empty,
		value = tracing::field::Empty,
	))]
	pub fn new<S>(name: S) -> Self
	where
		S: Into<String>,
	{
		let key = Uuid::new_v4();
		let name = name.into();

		tracing::Span::current()
			.record("name", &name)
			.record("value", format_args!("{key}"));

		tracing::debug!("generated API key");

		Self { key, name }
	}

	/// Returns the name of this key.
	pub fn name(&self) -> &str {
		&self.name
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for ApiKey {
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::api_key::from_request_parts",
		skip_all,
		fields(name = tracing::field::Empty, value = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self> {
		let key = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await?
			.token()
			.parse::<Uuid>()
			.map_err(|err| Error::invalid_api_key().context(err))?;

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
			true => Err(Error::expired_api_key()),
			false => Ok(ApiKey {
				key,
				name: row.name,
			}),
		})
		.ok_or_else(|| Error::invalid_api_key())??;

		tracing::Span::current()
			.record("name", api_key.name())
			.record("value", format_args!("{}", api_key.key));

		tracing::debug!("authenticated API key");

		Ok(api_key)
	}
}
