//! Opaque API keys for service authentication.
//!
//! These are for internal use, like GitHub actions. Each key has a unique name
//! that identifies it, and it is simply sent in a header.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

mod service;
pub use service::{ApiKeyLayer, ApiKeyService};

/// An opaque API key.
#[derive(Clone, Copy, Serialize, Deserialize)]
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

impl fmt::Display for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(self.0.as_hyphenated(), f)
	}
}

impl fmt::Debug for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_tuple("ApiKey")
			.field(self.0.as_hyphenated())
			.finish()
	}
}

impl FromStr for ApiKey
{
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		s.parse::<Uuid>().map(Self)
	}
}

crate::macros::sqlx_scalar_forward!(ApiKey as Hyphenated => {
	encode: |self| { *self.0.as_hyphenated() },
	decode: |uuid| { Self(Uuid::from(uuid)) },
});
