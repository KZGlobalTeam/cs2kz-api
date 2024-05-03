//! Logs emitted to stderr.

use std::io;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Returns a tracing layer for writing logs to stderr.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	tracing_subscriber::fmt::layer()
		.with_target(true)
		.with_writer(io::stderr)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.pretty()
		.with_filter(EnvFilter::from_default_env())
}
