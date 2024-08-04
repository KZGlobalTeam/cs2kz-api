//! This module contains various utility macros.

mod make_id;
pub(crate) use make_id::make_id;

mod sqlx_scalar_forward;
pub(crate) use sqlx_scalar_forward::sqlx_scalar_forward;
