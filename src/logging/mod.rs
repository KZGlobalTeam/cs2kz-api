//! Everything related to logging.

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod stderr;
mod files;

/// Initializes the global tracing subscriber.
pub fn init() -> anyhow::Result<WorkerGuard> {
	let (files_layer, guard, log_dir) = files::layer().context("files layer")?;
	let registry = tracing_subscriber::registry()
		.with(stderr::layer())
		.with(files_layer);

	#[cfg(feature = "console")]
	let registry = {
		use tracing_subscriber::{EnvFilter, Layer};
		registry.with(console_subscriber::spawn().with_filter(EnvFilter::new("tokio=trace")))
	};

	registry.init();

	tracing::info! {
		target: "cs2kz_api::audit_log",
		dir = %log_dir.display(),
		"initialized logging",
	};

	Ok(guard)
}
