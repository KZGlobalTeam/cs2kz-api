//! This module contains helpers for unit/integration tests.

use std::sync::Arc;

use color_eyre::eyre::WrapErr;
use cs2kz::SteamID;
use serde::de::DeserializeOwned;
use sqlx::{MySql, Pool};
use url::Url;

use crate::services::{
	AuthService,
	BanService,
	MapService,
	PlayerService,
	ServerService,
	SteamService,
};

pub const ALPHAKEKS_ID: SteamID = match SteamID::new(76561198282622073_u64) {
	Some(id) => id,
	None => unreachable!(),
};

pub fn steam_svc() -> SteamService
{
	let http_client = reqwest::Client::new();
	let api_url = Arc::new(Url::parse("http://127.0.0.1").unwrap());
	let steam_api_key = String::new();

	SteamService::new(api_url, steam_api_key, None, None, http_client)
}

pub fn auth_svc(database: Pool<MySql>) -> AuthService
{
	let http_client = reqwest::Client::new();
	let steam_svc = steam_svc();
	let jwt_secret = String::from("Zm9vYmFyYmF6");
	let cookie_domain = String::from("localhost");

	AuthService::new(database, http_client, steam_svc, jwt_secret, cookie_domain).unwrap()
}

pub fn player_svc(database: Pool<MySql>) -> PlayerService
{
	let auth_svc = auth_svc(database.clone());
	let steam_svc = steam_svc();

	PlayerService::new(database, auth_svc, steam_svc)
}

pub fn map_svc(database: Pool<MySql>) -> MapService
{
	let auth_svc = auth_svc(database.clone());
	let steam_svc = steam_svc();

	MapService::new(database, auth_svc, steam_svc)
}

pub fn server_svc(database: Pool<MySql>) -> ServerService
{
	let auth_svc = auth_svc(database.clone());

	ServerService::new(database, auth_svc)
}

pub fn ban_svc(database: Pool<MySql>) -> BanService
{
	let auth_svc = auth_svc(database.clone());

	BanService::new(database, auth_svc)
}

pub async fn parse_body<T>(body: axum::body::Body) -> color_eyre::Result<T>
where
	T: DeserializeOwned,
{
	let bytes = axum::body::to_bytes(body, usize::MAX).await?;
	let parsed = serde_json::from_slice::<T>(&bytes).context("parse body")?;

	Ok(parsed)
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

	// so `#[sqlx::test]` can create new databases
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
			::color_eyre::eyre::bail!(
				"assertion `{} == {}` failed\n  lhs: {:?}\n  rhs: {:?}",
				stringify!($lhs),
				stringify!($rhs),
				$lhs,
				$rhs,
			);
		}
	};
}

macro_rules! assert_ne {
	($lhs:expr, $rhs:expr $(, $($msg:tt)*)?) => {
		if &$lhs == &$rhs {
			::color_eyre::eyre::bail!(
				"assertion `{} != {}` failed\n  lhs: {:?}\n  rhs: {:?}",
				stringify!($lhs),
				stringify!($rhs),
				$lhs,
				$rhs,
			);
		}
	};
}

macro_rules! assert_matches {
	($expr:expr, $pat:pat $(if $cond:expr)? $(, $($msg:tt)*)?) => {
		::color_eyre::eyre::ensure!(matches!($expr, $pat $(if $cond)? $(, $($msg)*)?))
	};
}

pub(crate) use {assert, assert_eq, assert_matches, assert_ne};
