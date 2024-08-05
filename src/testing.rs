//! This module contains helpers for unit/integration tests.

/// Global constructor that will run before tests.
#[ctor::ctor]
fn ctor()
{
	use std::env;

	use tracing_subscriber::fmt::format::FmtSpan;
	use tracing_subscriber::EnvFilter;
	use url::Url;

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

	if let Ok(database_url) = env::var("DATABASE_URL") {
		let mut database_url = database_url.parse::<Url>().unwrap();
		database_url.set_username("root").unwrap();
		env::set_var("DATABASE_URL", database_url.as_str());
	}
}

macro_rules! assert {
	($expr:expr $(, $($msg:tt)*)?) => {
		::color_eyre::eyre::ensure!($expr $(, $($msg)*)?)
	};
}

macro_rules! assert_matches {
	($expr:expr, $pat:pat $(if $cond:expr)? $(, $($msg:tt)*)?) => {
		::color_eyre::eyre::ensure!(matches!($expr, $pat $(if $cond)? $(, $($msg)*)?))
	};
}

pub(crate) use {assert, assert_matches};
