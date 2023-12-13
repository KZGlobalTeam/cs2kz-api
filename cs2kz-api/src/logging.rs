use std::io;

use time::macros::format_description;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Sets up a [tracing registry] that will pick up logs.
///
/// Currently these logs are only printed to stderr, but they can also be formatted as JSON to
/// save them to disk, a database, or some external service.
///
/// [tracing registry]: tracing_subscriber::Registry
pub fn init() {
	let format = format_description!("[hour]:[second].[subsecond digits:5]");
	let timer = UtcTime::new(format);
	let span_events = FmtSpan::NEW | FmtSpan::CLOSE;
	let filter = EnvFilter::from_default_env();
	let level = filter.to_string();
	let stderr = tracing_subscriber::fmt::layer()
		.pretty()
		.with_thread_ids(true)
		.with_writer(io::stderr)
		.with_timer(timer)
		.with_span_events(span_events)
		.with_filter(filter);

	tracing_subscriber::registry().with(stderr).init();

	info!(%level, "Initialized logging");
}
