//! This module contains helpers for unit/integration tests.

use std::sync::Arc;

use sqlx::{MySql, Pool};
use url::Url;

use crate::services::{AuthService, SteamService};

pub fn auth_svc(database: Pool<MySql>) -> AuthService
{
	let http_client = reqwest::Client::new();
	let api_url = Arc::new(Url::parse("http://127.0.0.1").unwrap());
	let steam_api_key = String::new();
	let steam_svc = SteamService::new(api_url, steam_api_key, None, None, http_client.clone());
	let jwt_secret = String::from("Zm9vYmFyYmF6");
	let cookie_domain = String::from("localhost");

	AuthService::new(database, http_client, steam_svc, jwt_secret, cookie_domain).unwrap()
}

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

macro_rules! assert_eq {
	($lhs:expr, $rhs:expr $(, $($msg:tt)*)?) => {
		if &$lhs != &$rhs {
			let lhs = stringify!($lhs);
			let rhs = stringify!($rhs);
			::color_eyre::eyre::bail!(
				"assertion `{lhs} == {rhs}` failed\n  lhs: {lhs}\n  rhs:{rhs}"
			);
		}
	};
}

macro_rules! assert_matches {
	($expr:expr, $pat:pat $(if $cond:expr)? $(, $($msg:tt)*)?) => {
		::color_eyre::eyre::ensure!(matches!($expr, $pat $(if $cond)? $(, $($msg)*)?))
	};
}

pub(crate) use {assert, assert_eq, assert_matches};
