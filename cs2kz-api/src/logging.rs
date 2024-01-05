use std::io;

use sqlx::MySqlConnection;
use time::macros::format_description;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn init(database: MySqlConnection) {
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

	let audit_logs = cs2kz_api::audit_logs::AuditLayer::new(database);

	tracing_subscriber::registry()
		.with(audit_logs)
		.with(stderr)
		.init();

	info!(%level, "Initialized logging");
}
