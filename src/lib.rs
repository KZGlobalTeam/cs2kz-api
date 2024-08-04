#![doc = include_str!("../README.md")]
#![allow(clippy::blocks_in_conditions)] // TODO: remove when tokio-rs/tracing#2912 is fixed

/*
 * CS2KZ API - the core infrastructure for CS2KZ.
 * Copyright (C) 2024  AlphaKeks <alphakeks@dawn>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see https://www.gnu.org/licenses.
 */

#[macro_use]
extern crate sealed;

#[macro_use]
extern crate pin_project;

mod docs;
mod macros;
mod net;
mod num;
mod serde;
mod time;
mod util;

#[cfg(test)]
mod testing;

pub mod database;
pub mod http;
pub mod middleware;
pub mod runtime;
pub mod services;
pub mod setup;
pub mod stats;
pub mod openapi;

/// A [`tower::MakeService`] that can be passed to [`axum::serve()`].
pub type Server =
	axum::extract::connect_info::IntoMakeServiceWithConnectInfo<axum::Router, std::net::SocketAddr>;

/// Initializes the API's services and returns a [`tower::MakeService`].
///
/// When the returned service is called, it will return a new service that can
/// handle an incoming connection.
///
/// You'll likely just pass the return value to [`axum::serve()`] to run the
/// server.
#[tracing::instrument(target = "cs2kz_api::runtime", name = "start", err(Debug))]
pub async fn server(config: runtime::Config) -> Result<Server, setup::Error>
{
	use std::sync::Arc;

	use self::services::{
		AdminService,
		AuthService,
		BanService,
		HealthService,
		JumpstatService,
		MapService,
		PlayerService,
		PluginService,
		RecordService,
		ServerService,
		SteamService,
	};

	let http_client = reqwest::Client::new();
	let database = sqlx::pool::PoolOptions::new()
		.min_connections(database::min_connections())
		.max_connections(database::max_connections())
		.connect(config.database_url.as_str())
		.await?;

	sqlx::migrate!("./database/migrations")
		.run(&database)
		.await?;

	let api_url = Arc::new(config.public_url);

	let steam_svc = SteamService::new(
		Arc::clone(&api_url),
		config.steam_api_key,
		config.workshop_artifacts_path,
		config.depot_downloader_path,
		http_client.clone(),
	);

	let auth_svc = AuthService::new(
		database.clone(),
		http_client.clone(),
		steam_svc.clone(),
		config.jwt_secret,
		config.cookie_domain,
	)?;

	let health_svc = HealthService::new();
	let player_svc = PlayerService::new(database.clone(), auth_svc.clone(), steam_svc.clone());
	let map_svc = MapService::new(database.clone(), auth_svc.clone(), steam_svc.clone());
	let server_svc = ServerService::new(database.clone(), auth_svc.clone());
	let record_svc = RecordService::new(database.clone(), auth_svc.clone());
	let jumpstat_svc = JumpstatService::new(database.clone(), auth_svc.clone());
	let ban_svc = BanService::new(database.clone(), auth_svc.clone());
	let admin_svc = AdminService::new(database.clone(), auth_svc.clone());
	let plugin_svc = PluginService::new(database.clone());

	let docs = docs::router();

	let panic_handler = middleware::panic_handler::layer();
	let logging = middleware::logging::layer!();

	let server = axum::Router::new()
		.merge(health_svc)
		.nest("/players", player_svc.into())
		.nest("/maps", map_svc.into())
		.nest("/servers", server_svc.into())
		.nest("/records", record_svc.into())
		.nest("/jumpstats", jumpstat_svc.into())
		.nest("/bans", ban_svc.into())
		.nest("/auth", auth_svc.into())
		.nest("/admins", admin_svc.into())
		.nest("/plugin", plugin_svc.into())
		.layer(panic_handler)
		.layer(logging)
		.merge(docs)
		.into_make_service_with_connect_info::<std::net::SocketAddr>();

	Ok(server)
}
