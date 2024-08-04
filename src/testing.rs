//! This module contains helpers for unit/integration tests.

/// Global constructor that will run before tests.
#[ctor::ctor]
fn ctor()
{
	use tracing_subscriber::fmt::format::FmtSpan;
	use tracing_subscriber::EnvFilter;

	color_eyre::install().expect("failed to install color-eyre");
	tracing_subscriber::fmt()
		.compact()
		.with_ansi(true)
		.with_file(true)
		.with_level(true)
		.with_line_number(true)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_target(true)
		.with_test_writer()
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_env_filter(EnvFilter::from_default_env())
		.init();
}
