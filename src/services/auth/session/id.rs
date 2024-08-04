//! Session IDs for user authentication.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// A session ID.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionID(Uuid);

impl SessionID
{
	/// Generates a new random ID.
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}
}

impl fmt::Display for SessionID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(self.0.as_hyphenated(), f)
	}
}

impl fmt::Debug for SessionID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_tuple("SessionID")
			.field(self.0.as_hyphenated())
			.finish()
	}
}

impl FromStr for SessionID
{
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		s.parse::<Uuid>().map(Self)
	}
}

crate::macros::sqlx_scalar_forward!(SessionID as Hyphenated => {
	encode: |self| { *self.0.as_hyphenated() },
	decode: |uuid| { Self(Uuid::from(uuid)) },
});
