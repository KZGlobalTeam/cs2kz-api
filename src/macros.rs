#[macro_export]
macro_rules! audit {
	( $( $arg:tt )* ) => {
		::tracing::info!(audit = true, $( $arg )*)
	};
}

#[macro_export]
macro_rules! audit_warn {
	( $( $arg:tt )* ) => {
		::tracing::warn!(audit = true, $( $arg )*)
	};
}

#[macro_export]
macro_rules! audit_error {
	( $( $arg:tt )* ) => {
		::tracing::error!(audit = true, $( $arg )*)
	};
}
