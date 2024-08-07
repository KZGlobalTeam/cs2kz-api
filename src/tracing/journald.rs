//! Tracing layer for logging to journald.

use color_eyre::eyre::WrapErr;
use cs2kz_api::runtime::config::TracingJournaldConfig;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to journald.
///
/// This is intended for production, where we run as a systemd service.
pub fn layer<S>(
	config: TracingJournaldConfig,
) -> color_eyre::Result<impl tracing_subscriber::Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	tracing_journald::layer()
		.map(|layer| layer.with_syslog_identifier(String::from("cs2kz-api")))
		.map(|layer| layer.with_filter(config.filter))
		.context("create journald layer")
}
