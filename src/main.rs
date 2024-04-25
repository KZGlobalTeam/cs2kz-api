//! The entrypoint for the API.

use cs2kz_api::API;
use eyre::{Context, Result};
use sqlx::{Connection, MySqlConnection};

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
	if dotenvy::dotenv().is_err() {
		eprintln!("WARNING: no `.env` file found");
	}

	let _guard = logging::init().context("initialize logging")?;
	let config = cs2kz_api::Config::new().context("load config")?;
	let mut connection = MySqlConnection::connect(config.database_url.as_str())
		.await
		.context("connect to database")?;

	sqlx::migrate!("./database/migrations")
		.run(&mut connection)
		.await
		.context("run migrations")?;

	drop(connection);

	API::run(config).await.context("run API")?;

	Ok(())
}
