use color_eyre::Result;
use cs2kz_api::{Config, API};
use tracing::info;

mod logging;

#[tokio::main]
async fn main() -> Result<()> {
	// Setup error handling
	color_eyre::install()?;

	// Load environment variables.
	//
	// If the `.env` file does not exist, it's not a fatal error, since the user could still
	// set all the variables manually.
	if let Err(err) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {err}");
		eprintln!("Did you forget to create one?");
	}

	// Setup logging
	crate::logging::init();

	// Load API configuration
	let config = Config::new().await?;

	info!(?config, "Loaded API configuration");

	// Run the API
	API::run(config).await?;

	Ok(())
}
