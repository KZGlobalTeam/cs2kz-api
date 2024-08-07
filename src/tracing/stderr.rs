//! Tracing layer for logging to stderr.

use std::io;

use cs2kz_api::runtime::config::TracingStderrConfig;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to stderr.
pub fn layer<S>(config: TracingStderrConfig) -> impl tracing_subscriber::Layer<S>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	tracing_subscriber::fmt::layer()
		.pretty()
		.with_ansi(config.ansi)
		.with_file(true)
		.with_level(true)
		.with_line_number(true)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_target(true)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_writer(io::stderr)
		.with_filter(config.filter)
}
