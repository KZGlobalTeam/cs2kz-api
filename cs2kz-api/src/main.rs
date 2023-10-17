// Copyright (C) AlphaKeks <alphakeks@dawn.sh>
//
// This is free software. You can redistribute it and / or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3
// of the License, or (at your option) any later version.
//
// You should have received a copy of the GNU General Public License along with this repository.
// If not, see <https://www.gnu.org/licenses/>.

use {
	crate::config::Config,
	axum::Server,
	color_eyre::eyre::Context,
	cs2kz_api::{state::AppState, API},
	std::{fmt::Write, net::SocketAddr},
	tracing::{info, warn},
};

mod config;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
	color_eyre::install()?;

	if dotenvy::dotenv().is_err() {
		warn!("Failed to load `.env` file");
	}

	let config = Config::load()?;

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
