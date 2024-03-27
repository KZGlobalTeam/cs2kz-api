use std::path::PathBuf;
use std::{env, fs, io};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub fn layer<S>() -> io::Result<(impl tracing_subscriber::Layer<S>, WorkerGuard, PathBuf)>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	let log_dir = env::var("LOG_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|_| PathBuf::from("/var/log/cs2kz-api"));

	if !log_dir.exists() {
		fs::create_dir_all(&log_dir)?;
	}

	let log_dir = log_dir.canonicalize()?;

	let (writer, guard) = tracing_appender::rolling::Builder::new()
		.rotation(Rotation::DAILY)
		.filename_suffix("log")
		.build(&log_dir)
		.map(tracing_appender::non_blocking)
		.expect("failed to initialize logger");

	let layer = BunyanFormattingLayer::new(String::from("cs2kz-api"), writer).with_filter(
		FilterFn::new(|metadata| {
			metadata.target().starts_with("audit_log") || metadata.target().starts_with("cs2kz_api")
		}),
	);

	Ok((layer, guard, log_dir))
}
