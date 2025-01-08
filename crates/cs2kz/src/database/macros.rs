/// Implements [`sqlx::Type`], [`sqlx::Encode`], and [`sqlx::Decode`] for a type.
///
/// # Example
///
/// ```
/// use crate::database;
///
/// struct MyBytes(Vec<u8>);
///
/// database::impl_traits!(MyBytes as [u8] => {
///     fn encode<'a>(self, out: &'a [u8]) {
///         out = &self.0;
///     }
///
///     fn decode(bytes: Vec<u8>) -> Result<Self, BoxError> {
///         Ok(Self(bytes))
///     }
/// });
/// ```
pub(crate) macro impl_traits {
    ($ty:ty as $repr:ty => {
        fn encode($self:ident, $out:ident : $encode_ty:ty) {
            $($encode:stmt)*
        }

        fn decode($value:ident: $decode_ty:ty) -> Result<Self, BoxError>
            $decode:block
    }) => {
        $crate::database::impl_traits!($ty as $repr => {
            fn encode<'a>($self, $out : $encode_ty) {
                $($encode)*
            }

            fn decode<'r>($value : $decode_ty) -> Result<Self, BoxError> {
                $decode
            }
        });
    },
    ($ty:ty as $repr:ty => {
        fn encode<$encode_lt:lifetime>($self:ident, $out:ident : $encode_ty:ty) {
            $($encode:stmt)*
        }

        fn decode($value:ident: $decode_ty:ty) -> Result<Self, BoxError>
            $decode:block
    }) => {
        $crate::database::impl_traits!($ty as $repr => {
            fn encode<$encode_lt>($self, $out : $encode_ty) {
                $($encode)*
            }

            fn decode<'r>($value : $decode_ty) -> Result<Self, BoxError> {
                $decode
            }
        });
    },
    ($ty:ty as $repr:ty => {
        fn encode($self:ident, $out:ident : $encode_ty:ty) {
            $($encode:stmt)*
        }

        fn decode<$decode_lt:lifetime>($value:ident: $decode_ty:ty) -> Result<Self, BoxError>
            $decode:block
    }) => {
        $crate::database::impl_traits!($ty as $repr => {
            fn encode<'a>($self, $out : $encode_ty) {
                $($encode)*
            }

            fn decode<$decode_lt>($value : $decode_ty) -> Result<Self, BoxError> {
                $decode
            }
        });
    },
    ($ty:ty as $repr:ty => {
        fn encode<$encode_lt:lifetime>($self:ident, $out:ident : $encode_ty:ty) {
            $($encode:stmt)*
        }

        fn decode<$decode_lt:lifetime>($value:ident: $decode_ty:ty) -> Result<Self, BoxError>
            $decode:block
    }) => {
        impl<DB: sqlx::Database> sqlx::Type<DB> for $ty
        where
            $repr: sqlx::Type<DB>,
        {
            fn type_info() -> <DB as sqlx::Database>::TypeInfo {
                <$repr as sqlx::Type<DB>>::type_info()
            }

            fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
                <$repr as sqlx::Type<DB>>::compatible(ty)
            }
        }

        impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for $ty
        where
            for<$encode_lt> $encode_ty: sqlx::Encode<'q, DB>,
        {
            #[allow(redundant_semicolons, clippy::needless_late_init)]
            fn encode_by_ref(
                &$self,
                buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
                let $out;
                $($encode)*

                sqlx::Encode::<'q, DB>::encode_by_ref(&$out, buf).inspect_err(|error| {
                    error!(%error, "failed to encode value of type `{}`", stringify!($ty));
                })
            }

            #[allow(redundant_semicolons, clippy::needless_late_init)]
            fn produces(&$self) -> Option<<DB as sqlx::Database>::TypeInfo> {
                let $out;
                $($encode)*

                sqlx::Encode::<'q, DB>::produces(&$out)
            }

            #[allow(redundant_semicolons, clippy::needless_late_init)]
            fn size_hint(&$self) -> usize {
                let $out;
                $($encode)*

                sqlx::Encode::<'q, DB>::size_hint(&$out)
            }
        }

        impl<$decode_lt, DB: sqlx::Database> sqlx::Decode<$decode_lt, DB> for $ty
        where
            $decode_ty: sqlx::Decode<$decode_lt, DB>,
        {
            fn decode(
                value: <DB as sqlx::Database>::ValueRef<$decode_lt>,
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                <$decode_ty as sqlx::Decode<$decode_lt, DB>>::decode(value).and_then(|$value| $decode)
            }
        }
    },
}

/// Fetches a `COUNT(*)` from a table.
///
/// # Example
///
/// ```ignore
/// use crate::database;
///
/// let rows = database::count!(&database, "MyTable").await?;
/// let even_rows = database::count!(&database, "MyTable WHERE foo = ?", foo).await?;
/// ```
pub(crate) macro count($conn:expr, $($extra:tt)+) {
    async {
        let total: u64 = sqlx::query_scalar!("SELECT COUNT(*) FROM " + $($extra)+)
            .fetch_one($conn)
            .await?
            .try_into()
            .expect("`COUNT(…)` should not return a negative value");

        Ok::<u64, $crate::database::Error>(total)
    }
}
