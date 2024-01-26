use std::fmt;
use std::ops::Deref;
use std::result::Result as StdResult;
use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request, HeaderName, HeaderValue};
use axum_extra::headers::{self, Header};
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlExecutor, Transaction};
use tracing::trace;
use utoipa::ToSchema;

use crate::auth::{Role, RoleFlags};
use crate::{audit, Error, Result};

/// A known "service" that can authorize users with the API.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Service<const REQUIRED_FLAGS: u32 = 0> {
	/// The service's unique ID.
	pub id: u64,

	/// The service's name.
	pub name: String,

	/// The service URL.
	pub key: ServiceKey,

	/// The role flags this service is allowed to act as.
	#[serde(rename(serialize = "roles"))]
	pub role_flags: RoleFlags,
}

impl<const REQUIRED_FLAGS: u32> Service<REQUIRED_FLAGS> {
	/// Creates a new service in the database.
	pub async fn new(
		name: impl Into<String>,
		role_flags: RoleFlags,
		transaction: &mut Transaction<'static, MySql>,
	) -> Result<Self> {
		let name = name.into();
		let key = ServiceKey::new();

		sqlx::query! {
			r#"
			INSERT INTO
			  Services (name, `key`, role_flags)
			VALUES
			  (?, ?, ?)
			"#,
			name,
			key,
			role_flags,
		}
		.execute(transaction.as_mut())
		.await?;

		let id = sqlx::query!("SELECT LAST_INSERT_ID() id")
			.fetch_one(transaction.as_mut())
			.await
			.map(|row| row.id)?;

		audit!("service created", %id, %name);

		Ok(Self { id, name, key, role_flags })
	}

	pub async fn from_key(key: ServiceKey, executor: impl MySqlExecutor<'_>) -> Result<Self> {
		sqlx::query! {
			r#"
			SELECT
			  id,
			  name,
			  role_flags `role_flags: RoleFlags`
			FROM
			  Services
			WHERE
			  `key` = ?
			  AND (role_flags & ?) = ?
			"#,
			key,
			REQUIRED_FLAGS,
			REQUIRED_FLAGS,
		}
		.fetch_optional(executor)
		.await?
		.map(|row| Self { id: row.id, name: row.name, key, role_flags: row.role_flags })
		.ok_or_else(|| {
			trace!(%key, "unknown service");
			Error::Unauthorized
		})
	}
}

#[async_trait]
impl<const REQUIRED_FLAGS: u32> FromRequestParts<Arc<crate::State>> for Service<REQUIRED_FLAGS> {
	type Rejection = Error;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &Arc<crate::State>,
	) -> Result<Self> {
		let TypedHeader(key) = TypedHeader::<ServiceKey>::from_request_parts(parts, state).await?;

		Self::from_key(key, state.database()).await
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(transparent)]
pub struct ServiceKey(u32);

impl ServiceKey {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Self(rand::random())
	}
}

impl Deref for ServiceKey {
	type Target = u32;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl fmt::Display for ServiceKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&self.0, f)
	}
}

static SERVICE_KEY_HEADER_NAME: HeaderName = HeaderName::from_static("kz-service-key");

impl Header for ServiceKey {
	fn name() -> &'static HeaderName {
		&SERVICE_KEY_HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> StdResult<Self, headers::Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		values
			.next()
			.ok_or(headers::Error::invalid())?
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.parse()
			.map(Self)
			.map_err(|_| headers::Error::invalid())
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		let key = self.0.to_string();
		let value = HeaderValue::from_bytes(key.as_bytes()).expect("a u64 is valid utf8");

		values.extend([value]);
	}
}

/// A new [`Service`].
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewService {
	/// The service's name.
	pub name: String,

	/// The roles allowed to make requests from this service.
	pub roles: Vec<Role>,
}

/// A newly created [`Service`].
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedService {
	/// The service ID.
	pub service_id: u64,

	/// The generated servie key.
	pub service_key: ServiceKey,
}
