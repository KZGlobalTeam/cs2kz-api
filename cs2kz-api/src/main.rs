use cs2kz_api::state::AppState;
use cs2kz_api::API;

use crate::config::Config;

mod config;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	// Setup fatal error handling
	color_eyre::install()?;

	// Load `.env` variables
	if let Err(error) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {error:?}");
	}

	// Initialize logging
	cs2kz_api::logging::init();

	// Parse environment variables
	let config = Config::load()?;

	// Create application state
	let state = AppState::new(&config.database_url, &config.jwt_secret, config.public_url).await?;

	// Create axum router
	let router = API::router(state);

	// Run the server
	API::run(router, config.socket_addr).await?;

	Ok(())
}
