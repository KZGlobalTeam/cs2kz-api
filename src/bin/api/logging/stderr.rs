use std::io;

use time::macros::format_description;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::{FormatTime, UtcTime};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Provides a tracing layer for emitting logs to STDERR.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	tracing_subscriber::fmt::layer()
		.with_writer(io::stderr)
		.with_timer(timer())
		.with_span_events(FmtSpan::ACTIVE)
		.pretty()
		.with_filter(EnvFilter::from_default_env())
}

fn timer() -> impl FormatTime {
	let format = format_description!("[year]/[month]/[day]  [hour]:[second].[subsecond digits:5]");

	UtcTime::new(format)
}
