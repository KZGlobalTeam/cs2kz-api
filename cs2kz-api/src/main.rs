use color_eyre::eyre::Context;
use color_eyre::Result;
use cs2kz_api::{Config, API};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Connection, MySqlConnection};
use tokio::net::TcpListener;
use tracing::debug;

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

	// Load API configuration
	let config = Config::new().await?;

	// Setup logging
	let database = MySqlConnection::connect(config.database_url.as_str())
		.await
		.context("Failed to establish database connection.")?;

	crate::logging::init(database);

	// Connect to the database
	let database = MySqlPoolOptions::new()
		.connect(config.database_url.as_str())
		.await
		.context("Failed to establish database connection.")?;

	// Create TCP server
	let tcp_listener = TcpListener::bind(config.socket_addr)
		.await
		.context("Failed to bind TCP socket.")?;

	let socket_addr = tcp_listener
		.local_addr()
		.context("Failed to get local address for TCP socket.")?;

	debug!("Bound to TCP socket on {socket_addr}.");

	// Run the API
	API::run(config, database, tcp_listener).await?;

	Ok(())
}
