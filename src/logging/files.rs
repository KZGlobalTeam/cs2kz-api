//! Logs saved to files.

use std::path::PathBuf;
use std::{env, fs};

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer for saving logs to files.
///
/// # Returns
///
/// - the tracing layer
/// - guard that has to be kept around until the end of the program; for proper cleanup
/// - the path to the log directory
pub fn layer<S>() -> anyhow::Result<(impl tracing_subscriber::Layer<S>, WorkerGuard, PathBuf)>
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

	let layer = tracing_subscriber::fmt::layer()
		.with_target(true)
		.with_writer(writer)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.compact()
		.with_ansi(false)
		.with_filter(FilterFn::new(|metadata| {
			metadata.target().starts_with("audit_log") || metadata.target().starts_with("cs2kz_api")
		}));

	Ok((layer, guard, log_dir))
}
