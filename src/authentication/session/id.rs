//! Session IDs.

use derive_more::{Debug, Display, From, Into};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::MySql;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// A session ID.
///
/// This is a randomly generated UUID.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Into)]
#[debug("*****")]
#[display("{_0}")]
pub struct SessionID(Uuid);

impl SessionID {
	/// Generates a new random session ID.
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl sqlx::Type<MySql> for SessionID {
	fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
		<Hyphenated as sqlx::Type<MySql>>::type_info()
	}
}

impl<'q> sqlx::Encode<'q, MySql> for SessionID {
	fn encode_by_ref(&self, buf: &mut <MySql as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
		self.0.as_hyphenated().encode_by_ref(buf)
	}
}

impl<'r> sqlx::Decode<'r, MySql> for SessionID {
	fn decode(value: <MySql as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		<Hyphenated as sqlx::Decode<'r, MySql>>::decode(value)
			.map(Uuid::from)
			.map(Self)
	}
}
