//! Trait implementations for the [`sqlx`] crate.

/// [`sqlx`] impls that represent a [`SteamID`](crate::SteamID) as a 64-bit
/// integer.
#[cfg(not(feature = "sqlx-steamid-as-u32"))]
mod as_u64
{
	use std::borrow::Borrow;

	use crate::SteamID;

	impl<DB> sqlx::Type<DB> for SteamID
	where
		DB: sqlx::Database,
		u64: sqlx::Type<DB>,
	{
		fn type_info() -> <DB as sqlx::Database>::TypeInfo
		{
			<u64 as sqlx::Type<DB>>::type_info()
		}

		fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
		{
			<u64 as sqlx::Type<DB>>::compatible(ty)
		}
	}

	impl<'q, DB> sqlx::Encode<'q, DB> for SteamID
	where
		DB: sqlx::Database,
		u64: sqlx::Encode<'q, DB>,
	{
		fn encode_by_ref(
			&self,
			buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
		) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
		{
			<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(self.borrow(), buf)
		}

		fn encode(
			self,
			buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
		) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
		where
			Self: Sized,
		{
			<u64 as sqlx::Encode<'q, DB>>::encode(self.as_u64(), buf)
		}

		fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
		{
			<u64 as sqlx::Encode<'q, DB>>::produces(self.borrow())
		}

		fn size_hint(&self) -> usize
		{
			<u64 as sqlx::Encode<'q, DB>>::size_hint(self.borrow())
		}
	}

	impl<'r, DB> sqlx::Decode<'r, DB> for SteamID
	where
		DB: sqlx::Database,
		u64: sqlx::Decode<'r, DB>,
	{
		fn decode(
			value: <DB as sqlx::Database>::ValueRef<'r>,
		) -> Result<Self, sqlx::error::BoxDynError>
		{
			<u64 as sqlx::Decode<'r, DB>>::decode(value)?
				.try_into()
				.map_err(Into::into)
		}
	}
}

/// [`sqlx`] impls that represent a [`SteamID`](crate::SteamID) as a 32-bit
/// integer.
#[cfg(feature = "sqlx-steamid-as-u32")]
mod as_u32
{
	use crate::SteamID;

	impl<DB> sqlx::Type<DB> for SteamID
	where
		DB: sqlx::Database,
		u32: sqlx::Type<DB>,
	{
		fn type_info() -> <DB as sqlx::Database>::TypeInfo
		{
			<u32 as sqlx::Type<DB>>::type_info()
		}

		fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
		{
			<u32 as sqlx::Type<DB>>::compatible(ty)
		}
	}

	impl<'q, DB> sqlx::Encode<'q, DB> for SteamID
	where
		DB: sqlx::Database,
		u32: sqlx::Encode<'q, DB>,
	{
		fn encode_by_ref(
			&self,
			buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
		) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
		{
			<u32 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.as_u32(), buf)
		}

		fn encode(
			self,
			buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
		) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
		where
			Self: Sized,
		{
			<u32 as sqlx::Encode<'q, DB>>::encode(self.as_u32(), buf)
		}

		fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
		{
			<u32 as sqlx::Encode<'q, DB>>::produces(&self.as_u32())
		}

		fn size_hint(&self) -> usize
		{
			<u32 as sqlx::Encode<'q, DB>>::size_hint(&self.as_u32())
		}
	}

	impl<'r, DB> sqlx::Decode<'r, DB> for SteamID
	where
		DB: sqlx::Database,
		u32: sqlx::Decode<'r, DB>,
	{
		fn decode(
			value: <DB as sqlx::Database>::ValueRef<'r>,
		) -> Result<Self, sqlx::error::BoxDynError>
		{
			<u32 as sqlx::Decode<'r, DB>>::decode(value)?
				.try_into()
				.map_err(Into::into)
		}
	}
}
