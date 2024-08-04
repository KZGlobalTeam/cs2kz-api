//! API keys for CS2 servers.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// An API key for CS2 servers.
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct ApiKey(Uuid);

impl ApiKey
{
	/// Generates a new random key.
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}
}

impl fmt::Debug for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_tuple("ApiKey").field(&"*****").finish()
	}
}

impl fmt::Display for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "{}", self.0.as_hyphenated())
	}
}

crate::macros::sqlx_scalar_forward!(ApiKey as Hyphenated => {
	encode: |self| { *self.0.as_hyphenated() },
	decode: |uuid| { Self(Uuid::from(uuid)) },
});
