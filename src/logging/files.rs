//! Tracing layer for logging to files.

use std::path::PathBuf;
use std::{env, fs};

use color_eyre::eyre::WrapErr;
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
pub fn layer<S>() -> color_eyre::Result<(impl tracing_subscriber::Layer<S>, WorkerGuard)>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	let log_dir =
		env::var("LOG_DIR").map_or_else(|_| PathBuf::from("/var/log/cs2kz-api"), PathBuf::from);

	if !log_dir.exists() {
		fs::create_dir_all(&log_dir).context("create log dir")?;
	}

	let log_dir = log_dir
		.canonicalize()
		.context("canonicalize log dir path")?;

	let (writer, guard) = tracing_appender::rolling::Builder::new()
		.rotation(Rotation::DAILY)
		.filename_suffix("log")
		.build(&log_dir)
		.map(tracing_appender::non_blocking)
		.context("failed to initialize logger")?;

	let filter = super::default_env_filter()?;

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
		.with_filter(filter);

	Ok((layer, guard))
}
