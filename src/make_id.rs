//! Helper macro & trait to make "ID" types.
//!
//! Defining concrete types for different kinds of IDs makes it harder to accidentally mix them up.

use std::error::Error as StdError;

use thiserror::Error;

/// Extension trait to turn a raw integer into a specific ID type.
#[allow(private_bounds)] // this is intentional
pub trait IntoID: sealed::Sealed + Sized {
	/// Convert `self` into an `ID`.
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

/// An error for failed conversions from a raw integer to an ID.
#[derive(Debug, Error)]
#[error("failed to parse database ID")]
pub struct ConvertIDError<E>(E)
where
	E: StdError;

#[allow(clippy::missing_docs_in_private_items)]
mod sealed {
	pub(super) trait Sealed {}

	impl Sealed for u64 {}
}

/// A helper macro for defining an "ID" type.
///
/// All database tables with an `id` column get their own types defined by this macro in their
/// respective modules.
///
/// # Example
///
/// ```rust,ignore
/// // This will expand to a unit struct called `MapID` that wraps a `u16` and implements various
/// // traits so it can be treated like a `u16`, but still expresses a semantic difference.
/// make_id!(MapID as u16);
/// ```
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
