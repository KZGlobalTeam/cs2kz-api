//! The entrypoint for the API.

use std::error::Error;

use cs2kz_api::API;
use sqlx::{Connection, MySqlConnection};

mod logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	if dotenvy::dotenv().is_err() {
		eprintln!("WARNING: no `.env` file found");
	}

	let _guard = logging::init()?;
	let config = cs2kz_api::Config::new()?;
	let mut connection = MySqlConnection::connect(config.database_url.as_str()).await?;

	sqlx::migrate!("./database/migrations")
		.run(&mut connection)
		.await?;

	drop(connection);

	API::run(config).await?;

	Ok(())
}
