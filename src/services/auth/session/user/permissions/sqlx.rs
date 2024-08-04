//! Trait implementations for the [`sqlx`] crate.

use super::Permissions;

crate::macros::sqlx_scalar_forward!(Permissions as u64 => {
	encode: |self| { self.0 },
	decode: |value| { Self::new(value) },
});
