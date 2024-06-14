//! Copyright (C) 2024  AlphaKeks <alphakeks@dawn.sh>
//!
//! CS2KZ API.
//!
//! This is free software: you can redistribute it and/or modify it under
//! the terms of the GNU General Public License as published by the Free Software Foundation,
//! either version 3 of the License, or (at your option) any later version.
//!
//! This is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
//! without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
//! See the GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License along with this repository.
//! If not, see <https://www.gnu.org/licenses/>.

use std::panic;

use anyhow::Context;
use sqlx::{Connection, MySqlConnection};
use tracing::Instrument;

mod logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// `.env` files missing is not necessarily an issue (e.g. when running tests in CI), but we
	// log it to stderr just in case.
	if dotenvy::dotenv().is_err() {
		eprintln!("WARNING: no `.env` file found");
	}

	let _guard = logging::init().context("initialize logging")?;
	let runtime_span = tracing::info_span!("runtime::startup");

	let api_config = runtime_span
		.in_scope(cs2kz_api::Config::new)
		.context("load config")?;

	let mut connection = MySqlConnection::connect(api_config.database_url.as_str())
		.instrument(runtime_span.clone())
		.await
		.context("connect to database")?;

	// Run database migrations.
	//
	// If this fails, e.g. because the migration files have changed since they last have been
	// applied, the API will fail to startup, so the migrations can be fixed.
	sqlx::migrate!("./database/migrations")
		.run(&mut connection)
		.instrument(runtime_span.clone())
		.await
		.context("run migrations")?;

	// Don't wanna keep around a dead connection!
	drop(connection);

	let old_panic_hook = panic::take_hook();

	// If anything anywhere ever panics, we want to log it.
	panic::set_hook(Box::new(move |info| {
		tracing::error_span!("runtime::panic_hook").in_scope(|| {
			tracing::error!(target: "cs2kz_api::audit_log", message = %info);
		});

		old_panic_hook(info)
	}));

	cs2kz_api::run(api_config)
		.instrument(runtime_span)
		.await
		.context("run API")?;

	Ok(())
}
