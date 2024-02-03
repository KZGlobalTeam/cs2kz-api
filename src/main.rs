use std::error::Error as StdError;

use cs2kz_api::{Config, API};
use tracing::{info, warn};

mod logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
	// Load `.env` file.
	if let Err(error) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {error}");
	}

	// Load API configuration.
	let config = Config::new()?;

	// Initialize logging.
	logging::init(&config.database, config.axiom.clone()).await?;

	// Initialize the API.
	let api = API::new(config).await?;

	info!("Initialized API service.");

	for route in API::routes() {
		info!("Registered route: {route}");
	}

	let local_addr = api.local_addr()?;

	info!("Hosting SwaggerUI: <http://{local_addr}/docs/swagger-ui>");
	info!("Hosting OpenAPI Spec: <http://{local_addr}/docs/open-api.json>");

	if cfg!(not(feature = "production")) {
		warn!("running in development mode");
	}

	// Run the API.
	api.run().await;

	Ok(())
}
