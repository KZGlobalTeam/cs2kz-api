//! API keys for CS2 servers.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// An API key for CS2 servers.
#[derive(PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

impl<DB> sqlx::Type<DB> for ApiKey
where
	DB: sqlx::Database,
	Hyphenated: sqlx::Type<DB>,
	for<'a> &'a [u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<Hyphenated as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<Hyphenated as sqlx::Type<DB>>::compatible(ty) || <&[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for ApiKey
where
	DB: sqlx::Database,
	Hyphenated: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	{
		self.0.as_hyphenated().encode_by_ref(buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for ApiKey
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'r>)
	-> Result<Self, sqlx::error::BoxDynError>
	{
		let bytes = <&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)?;
		let uuid = Uuid::try_parse_ascii(bytes)?;

		Ok(Self(uuid))
	}
}
