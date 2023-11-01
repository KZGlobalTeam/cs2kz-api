use {
	crate::{args::Args, config::Config},
	axum::Server,
	color_eyre::eyre::Context,
	cs2kz_api::{state::AppState, API},
	std::{fmt::Write, net::SocketAddr},
	tracing::info,
};

mod config;
mod args;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	if let Err(error) = dotenvy::dotenv() {
		eprintln!("Failed to load `.env` file: {error:?}");
	}

	let mut config = Config::load()?;
	let args = Args::get();

	args.override_config(&mut config);

	if config.enable_logging {
		cs2kz_api::logging::init();
	}

	let state = AppState::new(&config.database_url).await?;
	let router = API::router(state).into_make_service_with_connect_info::<SocketAddr>();
	let server = Server::bind(&config.socket_addr()).serve(router);
	let addr = server.local_addr();

	info!("Listening on {addr}.");

	let routes = API::routes().fold(String::from("Registering routes:\n"), |mut routes, route| {
		writeln!(&mut routes, "\t\t\t\t\t• `{route}`").expect("This never fails.");
		routes
	});

	info!("{routes}");
	info!("SwaggerUI hosted at: <http://{addr}/api/docs/swagger-ui>");
	info!("OpenAPI spec hosted at: <http://{addr}/api/docs/openapi.json>");

	server.await.context("Failed to run axum.")?;

	Ok(())
}
