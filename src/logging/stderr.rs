//! Tracing layer for logging to stderr.

use std::io;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Creates a tracing layer that will emit logs to stderr.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	tracing_subscriber::fmt::layer()
		.with_target(true)
		.with_writer(io::stderr)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.pretty()
		.with_filter(if cfg!(feature = "production") {
			EnvFilter::new("cs2kz_api::audit_log=trace,warn")
		} else {
			EnvFilter::from_default_env()
		})
}
