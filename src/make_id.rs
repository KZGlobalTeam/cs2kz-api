//! A helper macro for creating "ID" types.

use std::error::Error as StdError;

use thiserror::Error;

/// A helper trait for converting raw database IDs into custom types created by [`make_id!()`].
///
/// [`make_id!()`]: crate::make_id!
#[allow(private_bounds)] // this is intentional
pub trait IntoID: private::Sealed + Sized {
	/// Converts the ID into some target type that can be conveniently specified via a
	/// turbofish.
	fn into_id<ID>(self) -> Result<ID, ConvertIDError<<Self as TryInto<ID>>::Error>>
	where
		Self: TryInto<ID>,
		<Self as TryInto<ID>>::Error: StdError;
}

impl IntoID for u64 {
	fn into_id<ID>(self) -> Result<ID, ConvertIDError<<Self as TryInto<ID>>::Error>>
	where
		Self: TryInto<ID>,
		<Self as TryInto<ID>>::Error: StdError,
	{
		self.try_into().map_err(ConvertIDError)
	}
}

/// An error that occurs when converting a raw database ID into a custom ID type.
#[derive(Debug, Error)]
#[error("failed to parse database ID")]
pub struct ConvertIDError<E>(E)
where
	E: StdError;

/// See <https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed>
mod private {
	/// See [module level documentation](self).
	pub(super) trait Sealed {}

	impl Sealed for u64 {}
}

/// Creates a thin integer wrapper that can be used as an ID with semantic meaning.
#[macro_export]
macro_rules! make_id {
	($name:ident as u64) => {
		$crate::make_id!(@private $name as u64);
	};
	($name:ident as $repr:ty) => {
		$crate::make_id!(@private $name as $repr);

		impl ::std::convert::TryFrom<u64> for $name {
			type Error = <$repr as ::std::convert::TryFrom<u64>>::Error;

			fn try_from(value: u64) -> ::std::result::Result<Self, Self::Error> {
				value.try_into().map(Self)
			}
		}
	};
	(@private $name:ident as $repr:ty) => {
		#[allow(missing_docs, clippy::missing_docs_in_private_items)]
		#[repr(transparent)]
		#[derive(
			Debug,
			Clone,
			Copy,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			::derive_more::Display,
			::derive_more::Into,
			::derive_more::From,
			::serde::Serialize,
			::serde::Deserialize,
			::sqlx::Type,
			::utoipa::ToSchema,
		)]
		#[serde(transparent)]
		#[sqlx(transparent)]
		#[display("{_0}")]
		pub struct $name(pub $repr);

		impl ::std::ops::Deref for $name {
			type Target = $repr;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}
	};
}
