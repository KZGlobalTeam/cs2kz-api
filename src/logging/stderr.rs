//! Tracing layer for logging to stderr.

use std::io;

use color_eyre::eyre::WrapErr;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Generates the default layer code for both local & production environments.
macro_rules! layer {
	() => {
		tracing_subscriber::fmt::layer()
			.compact()
			.with_ansi(!cfg!(feature = "production"))
			.with_file(true)
			.with_level(true)
			.with_line_number(true)
			.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
			.with_target(true)
			.with_thread_ids(false)
			.with_thread_names(true)
			.with_writer(io::stderr)
	};
}

/// Creates a tracing layer that will emit logs to stderr.
#[cfg(feature = "production")]
pub fn layer<S>() -> color_eyre::Result<impl tracing_subscriber::Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	filter().map(|filter| layer!().with_filter(filter))
}

/// Creates a tracing layer that will emit logs to stderr.
#[cfg(not(feature = "production"))]
pub fn layer<S>() -> color_eyre::Result<impl tracing_subscriber::Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	filter().map(|filter| layer!().pretty().with_filter(filter))
}

/// Returns the filter layer for the stderr layer.
fn filter() -> color_eyre::Result<EnvFilter>
{
	super::default_env_filter().and_then(|mut filter| {
		if cfg!(feature = "production") {
			let api_directive = "cs2kz_api=warn"
				.parse()
				.context("failed to parse env-filter directive")?;

			let sqlx_directive = "sqlx=warn"
				.parse()
				.context("failed to parse env-filter directive")?;

			filter = filter
				.add_directive(api_directive)
				.add_directive(sqlx_directive);
		}

		Ok(filter)
	})
}
