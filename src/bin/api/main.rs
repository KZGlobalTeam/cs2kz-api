use std::fmt::Write;

use color_eyre::eyre::Context;
use color_eyre::Result;
use cs2kz_api::{Config, API};
use sqlx::{Connection, MySqlConnection};
use tracing::{info, warn};

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
	if let Err(error) = dotenvy::dotenv() {
		eprintln!("WARN: Failed to load `.env` file: {error}");
	}

	let config = Config::new()?;
	let mut db_connection = MySqlConnection::connect(config.database.url.as_str())
		.await
		.context("failed to connect to database")?;

	sqlx::migrate!("./database/migrations")
		.run(&mut db_connection)
		.await
		.context("failed to run migrations")?;

	logging::init(db_connection);

	let api = API::new(config).await?;

	info!("Initialized API service.");

	let mut routes = String::from("\n");

	for route in API::routes() {
		writeln!(&mut routes, "\t\t\t\t\t\t• {route}")?;
	}

	info!("Registered API routes: {routes}");

	let local_addr = api.local_addr()?;

	info!("Hosting SwaggerUI: <http://{local_addr}/docs/swagger-ui>");
	info!("Hosting OpenAPI Spec: <http://{local_addr}/docs/open-api.json>");

	if cfg!(not(feature = "production")) {
		warn!("running in development mode");
	}

	api.run().await;

	Ok(())
}
