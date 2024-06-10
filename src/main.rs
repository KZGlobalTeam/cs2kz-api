//! The entrypoint for the API.

use std::panic;

use anyhow::Context;
use sqlx::{Connection, MySqlConnection};
use tracing::Instrument;

mod logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	if dotenvy::dotenv().is_err() {
		eprintln!("WARNING: no `.env` file found");
	}

	let _guard = logging::init().context("initialize logging")?;
	let runtime_span = tracing::info_span!("runtime::startup");
	let config = runtime_span.in_scope(|| cs2kz_api::Config::new().context("load config"))?;
	let mut connection = MySqlConnection::connect(config.database_url.as_str())
		.instrument(runtime_span.clone())
		.await
		.context("connect to database")?;

	sqlx::migrate!("./database/migrations")
		.run(&mut connection)
		.instrument(runtime_span.clone())
		.await
		.context("run migrations")?;

	drop(connection);

	let old_panic_hook = panic::take_hook();

	panic::set_hook(Box::new(move |info| {
		tracing::error_span!("runtime::panic_hook").in_scope(|| {
			tracing::error!(target: "cs2kz_api::audit_log", message = %info);
		});

		old_panic_hook(info)
	}));

	cs2kz_api::run(config)
		.instrument(runtime_span)
		.await
		.context("run API")?;

	Ok(())
}
