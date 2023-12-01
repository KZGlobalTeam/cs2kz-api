use {
	crate::{args::Args, config::Config},
	color_eyre::eyre::Context,
	cs2kz_api::{state::AppState, API},
	std::{fmt::Write, net::SocketAddr},
	tokio::net::TcpListener,
	tracing::info,
};

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
	let router = API::router(state).into_make_service_with_connect_info::<SocketAddr>();

	// Create HTTP server
	let tcp_listener = TcpListener::bind(config.socket_addr())
		.await
		.context("Failed to bind TCP listener.")?;

	// Print information to stdout
	let addr = tcp_listener.local_addr()?;

	info!("Listening on {addr}.");

	let routes = API::routes().fold(String::from("Registering routes:\n"), |mut routes, route| {
		writeln!(&mut routes, "\t\t\t\t\t• `{route}`").expect("This never fails.");
		routes
	});

	info!("{routes}");
	info!("SwaggerUI hosted at: <http://{addr}/api/docs/swagger-ui>");
	info!("OpenAPI spec hosted at: <http://{addr}/api/docs/openapi.json>");

	// Run the server
	axum::serve(tcp_listener, router)
		.await
		.context("Failed to run axum.")?;

	Ok(())
}
