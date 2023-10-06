//! Logging related functions.

use {
	tracing::info,
	tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

/// The default log level.
///
/// This will be used if `RUST_LOG` was not specified.
const DEFAULT_FILTER: &str = "WARN,cs2kz_api=TRACE";

/// Will initialize logging.
pub fn init() {
	// Write logs to STDERR.
	let writer = std::io::stderr;

	// Try to get `RUST_LOG` from the environment. Otherwise fall back to some default value.
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());
	let level = env_filter.to_string();

	// Which [`tracing::instrument`] events to log.
	let span_events = FmtSpan::ENTER;

	tracing_subscriber::fmt()
		.pretty()
		.with_writer(writer)
		.with_env_filter(env_filter)
		.with_span_events(span_events)
		.init();

	info!(%level, "Initialized logging");
}
