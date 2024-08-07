//! Tracing layer for logging to files.

use std::fs;

use color_eyre::eyre::WrapErr;
use cs2kz_api::runtime::config::TracingFilesConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to files.
///
/// The returned [`WorkerGuard`] must be kept alive so it can perform cleanup
/// when the application shuts down.
///
/// The returned [`PathBuf`] is the directory storing the log files.
pub fn layer<S>(
	config: TracingFilesConfig,
) -> color_eyre::Result<(impl tracing_subscriber::Layer<S>, WorkerGuard)>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	if !config.path.exists() {
		fs::create_dir_all(&config.path).context("create log dir")?;
	}

	let log_dir = config
		.path
		.canonicalize()
		.context("canonicalize log dir path")?;

	let (writer, guard) = tracing_appender::rolling::Builder::new()
		.rotation(Rotation::DAILY)
		.filename_suffix("log")
		.build(&log_dir)
		.map(tracing_appender::non_blocking)
		.context("failed to initialize logger")?;

	let layer = tracing_subscriber::fmt::layer()
		.compact()
		.with_ansi(false)
		.with_file(true)
		.with_level(true)
		.with_line_number(true)
		.with_span_events(FmtSpan::FULL)
		.with_target(true)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_writer(writer)
		.with_filter(config.filter);

	Ok((layer, guard))
}
