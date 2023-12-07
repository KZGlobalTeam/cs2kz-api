use cs2kz_api::state::AppState;
use cs2kz_api::API;

use crate::args::Args;
use crate::config::Config;

mod config;
mod args;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	// Setup fatal error handling
	color_eyre::install()?;

	// Load `.env` variables
	if let Err(error) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {error:?}");
	}

	// Parse environment variables
	let mut config = Config::load()?;

	// Parse CLI arguments
	Args::get().override_config(&mut config);

	// Initialize logging
	if config.enable_logging {
		cs2kz_api::logging::init();
	}

	// Create application state
	let state = AppState::new(&config.database_url, &config.jwt_secret).await?;

	// Create axum router
	let router = API::router(state);

	// Run the server
	API::run(router, config.socket_addr()).await?;

	Ok(())
}
