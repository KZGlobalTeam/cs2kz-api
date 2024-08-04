//! This module contains the [`make_id!()`] macro, which will generate the
//! boilerplate for an "ID"-like type.
//!
//! Most database tables have IDs, and using raw integers makes it easy to mix
//! them up accidentally. [`make_id!()`] will generate a wrapper type for every
//! unique ID type you need, so you can't mix them up!

/// Creates a new "ID" type.
///
/// This will produce a thin wrapper around a specified integer type, that
/// implements all the typical traits you'd expect.
///
/// # Example
///
/// ```ignore
/// crate::macros::make_id! {
///     /// Some useful documentation.
///     MyID as u16
/// }
/// ```
macro_rules! make_id {
	($(#[$meta:meta])* $name:ident as u64) => {
		$crate::macros::make_id!(@private $(#[$meta])* $name as u64);
	};

	($(#[$meta:meta])* $name:ident as $repr:ty) => {
		$crate::macros::make_id!(@private $(#[$meta])* $name as $repr);

		impl TryFrom<u64> for $name
		{
			type Error = <$repr as TryFrom<u64>>::Error;

			fn try_from(value: u64) -> std::result::Result<Self, Self::Error>
			{
				<$repr>::try_from(value).map(Self)
			}
		}
	};

	(@private $(#[$meta:meta])* $name:ident as $repr:ty) => {
		$(#[$meta])*
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
			serde::Serialize,
			serde::Deserialize,
			sqlx::Type,
			utoipa::ToSchema,
		)]
		#[serde(transparent)]
		#[sqlx(transparent)]
		#[schema(value_type = $repr)]
		pub struct $name(pub $repr);

		impl std::fmt::Display for $name
		{
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				std::fmt::Display::fmt(&self.0, f)
			}
		}

		impl std::ops::Deref for $name
		{
			type Target = $repr;

			fn deref(&self) -> &Self::Target
			{
				&self.0
			}
		}

		impl From<$name> for $repr
		{
			fn from(value: $name) -> Self
			{
				value.0
			}
		}

		impl From<$repr> for $name
		{
			fn from(value: $repr) -> Self
			{
				Self(value)
			}
		}

		impl std::str::FromStr for $name
		{
			type Err = <$repr as std::str::FromStr>::Err;

			fn from_str(s: &str) -> std::result::Result<Self, Self::Err>
			{
				<$repr as std::str::FromStr>::from_str(s).map(Self)
			}
		}
	};
}

pub(crate) use make_id;
