pub mod web;

/// Convenience macro for generating middleware layers.
macro_rules! layer {
	( $($role:ident),+ with $state:expr ) => (|| {
		::axum::middleware::from_fn_with_state(
			$state,
			$crate::middleware::auth::web::layer::<{
				0 $( | $crate::auth::Role::$role as u32 )+
			}>,
		)
	});
}

pub(crate) use layer;
