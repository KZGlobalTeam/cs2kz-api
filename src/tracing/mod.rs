//! Trace capturing facilities.

use color_eyre::eyre::WrapErr;
use cs2kz_api::runtime::config::TracingConfig;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

mod stderr;
mod files;

#[cfg(target_os = "linux")]
mod journald;

#[cfg(feature = "console")]
mod console;

#[derive(Debug)]
pub struct Guard
{
	/// The guard returned by [`tracing-appender`]'s logging thread.
	#[allow(dead_code)]
	appender_guard: tracing_appender::non_blocking::WorkerGuard,
}

/// Initializes [`tracing-subscriber`].
///
/// NOTE: the returned [`Guard`] will perform cleanup for the tracing layer that
/// emits logs to files, which means it has to stay alive until the program
/// exits!
pub fn init(config: TracingConfig) -> color_eyre::Result<Option<Guard>>
{
	if !config.enable {
		return Ok(None);
	}

	let stderr = config.stderr.enable.then(|| stderr::layer(config.stderr));
	let (files, appender_guard) =
		files::layer(config.files).context("initialize files tracing layer")?;

	let layer = Layer::and_then(stderr, files);

	#[cfg(target_os = "linux")]
	let layer = {
		let journald = config
			.journald
			.enable
			.then(|| journald::layer(config.journald))
			.transpose()?;

		Layer::and_then(layer, journald)
	};

	let registry = tracing_subscriber::registry().with(layer.with_filter(config.filter));

	#[cfg(feature = "console")]
	let registry = registry.with(console::layer(config.console)?);

	registry.init();

	tracing::info!("initialized tracing");

	Ok(Some(Guard { appender_guard }))
}
