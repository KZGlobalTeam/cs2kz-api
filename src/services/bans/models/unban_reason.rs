//! Reasons for which players can get unbanned.

use std::convert::Infallible;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Reasons for which players can get unbanned.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum UnbanReason
{
	/// The ban was a false ban.
	FalseBan,

	/// Some other reason.
	Other(String),
}

impl UnbanReason
{
	/// Returns a string representation of this [`UnbanReason`].
	pub fn as_str(&self) -> &str
	{
		match self {
			Self::FalseBan => "false_ban",
			Self::Other(other) => other.as_str(),
		}
	}
}

impl FromStr for UnbanReason
{
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		match s {
			"false_ban" => Ok(Self::FalseBan),
			other => Ok(Self::Other(other.to_owned())),
		}
	}
}

crate::macros::sqlx_scalar_forward!(UnbanReason as String => {
	encode: |self| { self.as_str().to_owned() },
	decode: |value| { value.parse()? },
});
