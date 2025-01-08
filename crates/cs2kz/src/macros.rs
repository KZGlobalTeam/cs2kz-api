//! Utility macros.
//!
//! This module is declared with `#[macro_use]`, so every macro defined in here is in-scope by
//! default in every other module of this crate.

/// Defines an "ID" type.
///
/// # Example
///
/// ```ignore
/// define_id_type! {
///     /// Useful documentation.
///     #[derive(…)]
///     pub struct MyId(u64);
/// }
/// ```
macro_rules! define_id_type {
    {
        $(#[$meta:meta])*
        $vis:vis struct $name:ident( $(#[$inner_meta:meta])* $inner:ty);

        $($item:item)*
    } => {
        $(#[$meta])*
        #[derive(
            Debug,
            Display,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        $vis struct $name( $(#[$inner_meta])* $inner);

        impl $name {
            pub fn from_inner(value: impl Into<$inner>) -> Self {
                Self(value.into())
            }

            pub fn into_inner(self) -> $inner {
                self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = <$inner as std::str::FromStr>::Err;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                <$inner as std::str::FromStr>::from_str(value).map($name)
            }
        }

        $($item)*
    };
}
