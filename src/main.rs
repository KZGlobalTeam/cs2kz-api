use std::error::Error as StdError;
use std::fmt::Write;

use cs2kz_api::{Config, API};
use sqlx::MySqlPool;
use tracing::{info, warn};

mod logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
	if let Err(error) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {error}");
	}

	let config = Config::new()?;
	let audit_log_db = MySqlPool::connect(config.database.url.as_str()).await?;

	logging::init(audit_log_db, config.axiom.clone());

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
