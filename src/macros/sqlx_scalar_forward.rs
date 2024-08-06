//! This module contains the `sqlx_scalar_foward!()` macro, which makes it
//! easy to implement [`sqlx::Type`], [`sqlx::Encode`], and [`sqlx::Decode`] in
//! terms of an existing type.

/// Forward [`sqlx::Type`], [`sqlx::Encode`], and [`sqlx::Decode`]
/// implementations to an existing type, only specifying the direct
/// encoding/decoding process.
///
/// # Example
///
/// ```ignore
/// struct MyWrapper(u64);
///
/// crate::macros::sqlx_scalar_forward!(MyWrapper as u64 => {
///     encode: |this| { this.0 },
///     decode: |value| { Self(value) },
/// });
/// ```
macro_rules! sqlx_scalar_forward {
	($ty:ty as $as:ty => {
		encode: |$self:ident| $encode_impl:block,
		decode: |$value:pat_param| $decode_impl:block,
	}) => {
		impl<DB> sqlx::Type<DB> for $ty
		where
			DB: sqlx::Database,
			$as: sqlx::Type<DB>,
		{
			fn type_info() -> <DB as sqlx::Database>::TypeInfo
			{
				<$as as sqlx::Type<DB>>::type_info()
			}

			fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
			{
				dbg!(ty);
				dbg!(<$as as sqlx::Type<DB>>::type_info());
				<$as as sqlx::Type<DB>>::compatible(ty)
			}
		}

		impl<'q, DB> sqlx::Encode<'q, DB> for $ty
		where
			DB: sqlx::Database,
			$as: sqlx::Encode<'q, DB>,
		{
			fn encode_by_ref(
				&$self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
			{
				let value = $encode_impl;

				<$as as sqlx::Encode<'q, DB>>::encode_by_ref(&value, buf)
			}

			fn encode(
				$self,
				buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
			) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
			where
				Self: Sized,
			{
				<$as as sqlx::Encode<'q, DB>>::encode($encode_impl, buf)
			}

			fn produces(&$self) -> Option<<DB as sqlx::Database>::TypeInfo>
			{
				let value = $encode_impl;

				<$as as sqlx::Encode<'q, DB>>::produces(&value)
			}

			fn size_hint(&$self) -> usize
			{
				let value = $encode_impl;

				<$as as sqlx::Encode<'q, DB>>::size_hint(&value)
			}
		}

		impl<'r, DB> sqlx::Decode<'r, DB> for $ty
		where
			DB: sqlx::Database,
			$as: sqlx::Decode<'r, DB>,
		{
			fn decode(
				value: <DB as sqlx::Database>::ValueRef<'r>,
			) -> Result<Self, sqlx::error::BoxDynError>
			{
				let $value = <$as as sqlx::Decode<'r, DB>>::decode(value)?;

				Ok($decode_impl)
			}
		}
	};
}

pub(crate) use sqlx_scalar_forward;
