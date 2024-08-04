//! Trait implementations for the [`sqlx`] crate.

use crate::Mode;

impl<DB> sqlx::Type<DB> for Mode
where
	DB: sqlx::Database,
	u8: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<u8 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<u8 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Mode
where
	DB: sqlx::Database,
	u8: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	{
		<u8 as sqlx::Encode<'q, DB>>::encode_by_ref(&u8::from(*self), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
	where
		Self: Sized,
	{
		<u8 as sqlx::Encode<'q, DB>>::encode(u8::from(self), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		<u8 as sqlx::Encode<'q, DB>>::produces(&u8::from(*self))
	}

	fn size_hint(&self) -> usize
	{
		<u8 as sqlx::Encode<'q, DB>>::size_hint(&u8::from(*self))
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Mode
where
	DB: sqlx::Database,
	u8: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'r>)
	-> Result<Self, sqlx::error::BoxDynError>
	{
		<u8 as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map_err(Into::into)
	}
}
