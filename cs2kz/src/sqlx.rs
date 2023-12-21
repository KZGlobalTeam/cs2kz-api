#[rustfmt::skip]
macro_rules! from_row_as {
	(
		$type:ty as $db_type:ty {
			encode: |$encode:pat_param| $encode_impl:block
			decode: |$decode:pat_param| $decode_impl:block
		}
	) => {
		impl<DB: sqlx::Database> sqlx::Type<DB> for $type
		where
			$db_type: sqlx::Type<DB>,
		{
			fn type_info() -> <DB as sqlx::Database>::TypeInfo {
				<$db_type as sqlx::Type<DB>>::type_info()
			}
		}

		impl<'query, DB: sqlx::Database> sqlx::Encode<'query, DB> for $type
		where
			$db_type: sqlx::Encode<'query, DB>,
		{
			fn encode_by_ref(&self, buf: &mut <DB as sqlx::database::HasArguments<'query>>::ArgumentBuffer) -> sqlx::encode::IsNull {
				let $encode = self;
				<$db_type as sqlx::Encode<'query, DB>>::encode($encode_impl, buf)
			}
		}

		impl<'row, DB: sqlx::Database> sqlx::Decode<'row, DB> for $type
		where
			$db_type: sqlx::Decode<'row, DB>,
		{
			fn decode(value: <DB as sqlx::database::HasValueRef<'row>>::ValueRef) -> std::result::Result<Self, sqlx::error::BoxDynError> {
				let $decode = <$db_type as sqlx::Decode<'row, DB>>::decode(value)?;
				($decode_impl).map_err(Into::into)
			}
		}
	};
}

pub(crate) use from_row_as;
