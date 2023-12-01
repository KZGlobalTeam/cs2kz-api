//! Logging related functions.

use {
	std::io::Stderr,
	tracing::info,
	tracing_subscriber::{fmt::format::FmtSpan, EnvFilter},
};

/// The default log level.
///
/// This will be used if `RUST_LOG` was not specified.
static DEFAULT_FILTER: &str = "WARN,cs2kz_api=TRACE,sqlx=DEBUG";

/// The default writer.
///
/// This is where logs will end up.
static DEFAULT_WRITER: fn() -> Stderr = std::io::stderr;

/// Will initialize logging.
pub fn init() {
	// Try to get `RUST_LOG` from the environment. Otherwise fall back to some default value.
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());
	let level = env_filter.to_string();

	// Which `tracing::instrument` events to log.
	let span_events = FmtSpan::NEW | FmtSpan::CLOSE;

	tracing_subscriber::fmt()
		.pretty()
		.with_writer(DEFAULT_WRITER)
		.with_span_events(span_events)
		.with_env_filter(env_filter)
		.init();

	info!(%level, "Initialized logging");
}
