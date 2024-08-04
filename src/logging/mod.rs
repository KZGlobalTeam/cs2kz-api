//! Log-capturing facilities.

use std::env;

use color_eyre::eyre::WrapErr;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod stderr;
mod files;

#[cfg(feature = "console")]
mod console;

/// RAII guard that will perform cleanup on drop.
#[allow(dead_code)]
pub struct Guard
{
	/// The guard returned by [`tracing-appender`]'s logging thread.
	appender_guard: tracing_appender::non_blocking::WorkerGuard,
}

/// Initializes [`tracing-subscriber`].
///
/// NOTE: the returned [`Guard`] will perform cleanup for the tracing layer that
/// emits logs to files, which means it has to stay alive until the program
/// exits!
pub fn init() -> color_eyre::Result<Guard>
{
	let stderr = stderr::layer().context("construct logging layer for stderr")?;
	let (files, appender_guard) = files::layer().context("construct logging layer for files")?;
	let registry = tracing_subscriber::registry().with(stderr).with(files);

	#[cfg(feature = "console")]
	let registry = registry.with(console::layer()?);

	registry.init();

	tracing::info!("initialized logging");

	Ok(Guard { appender_guard })
}

/// Returns a default filter layer that all layers should use by default.
fn default_env_filter() -> color_eyre::Result<EnvFilter>
{
	let filter = EnvFilter::new(concat!(
		"cs2kz_api=debug",
		",cs2kz_api::audit_log=trace",
		",cs2kz_api::runtime=info",
		",sqlx=debug",
		",warn",
	));

	let Ok(custom) = env::var("RUST_LOG") else {
		return Ok(filter);
	};

	custom.split(',').try_fold(filter, |filter, raw_directive| {
		raw_directive
			.parse()
			.map(|directive| filter.add_directive(directive))
			.context("parse env-filter directive")
	})
}
