/* Copyright (C) 2024  AlphaKeks <alphakeks@dawn.sh>
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this repository.  If not, see <https://www.gnu.org/licenses/>.
 */

#![feature(decl_macro)]
#![feature(future_join)]
#![feature(iter_chain)]
#![feature(let_chains)]
#![feature(panic_payload_as_str)]
#![feature(panic_update_hook)]
#![feature(return_type_notation)]

#[macro_use]
extern crate derive_more;

#[macro_use(trace, debug, info, info_span, warn, error)]
extern crate tracing;

#[macro_use(select, try_join)]
extern crate tokio;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{future, io};

use axum::{Router, ServiceExt, routing};
use cs2kz::Context;
use futures_util::FutureExt as _;
use tokio::sync::oneshot;
use tokio::task;
use tokio_util::time::FutureExt as _;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tower_http::metrics::InFlightRequestsLayer;

#[macro_use]
mod macros;

pub mod config;
pub use config::Config;

pub mod runtime;
pub mod openapi;

pub mod plugin;
pub mod users;
pub mod servers;
pub mod players;
pub mod maps;
pub mod jumpstats;
pub mod records;
pub mod bans;

mod extract;
mod problem_details;
mod replays;
mod response;
mod serde;
mod steam;

mod auth;
mod metrics;
mod middleware;
mod ws;

cfg_taskdump! {
    mod taskdump;
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display("failed to initialize runtime: {_0}")]
    #[from(ignore)]
    InitializeRuntime(io::Error),

    #[display("{_0}")]
    InitializeContext(cs2kz::context::InitializeContextError),

    #[display("failed to run server: {_0}")]
    #[from(ignore)]
    RunServer(io::Error),
}

/// Run the API.
///
/// This function will initialize its own [`tokio`] runtime and **block** until the server shuts
/// down.
pub fn run(config: Config) -> Result<(), Error> {
    runtime::build(&config.runtime)
        .map_err(Error::InitializeRuntime)?
        .block_on(async {
            let cx = Context::new(config.cs2kz).await?;
            let server_config = Arc::new(config.server);
            let cookie_config = Arc::new(config.cookies);
            let steam_auth_config = Arc::new(config.steam_auth);
            let depot_downloader_config = Arc::new(config.depot_downloader);

            let router = Router::new()
                .route("/", routing::get("(͡ ͡° ͜ つ ͡͡°)"))
                .nest("/docs", openapi::router(Arc::clone(&server_config)))
                .nest("/plugin", plugin::router(cx.clone(), &config.access_keys))
                .nest("/users", users::router(cx.clone(), Arc::clone(&cookie_config)))
                .nest(
                    "/auth",
                    auth::router(
                        cx.clone(),
                        Arc::clone(&steam_auth_config),
                        Arc::clone(&cookie_config),
                    ),
                )
                .nest("/servers", servers::router(cx.clone(), Arc::clone(&cookie_config)))
                .nest(
                    "/players",
                    players::router(
                        cx.clone(),
                        Arc::clone(&steam_auth_config),
                        Arc::clone(&cookie_config),
                    ),
                )
                .nest(
                    "/maps",
                    maps::router(
                        cx.clone(),
                        Arc::clone(&cookie_config),
                        Arc::clone(&steam_auth_config),
                        depot_downloader_config,
                    ),
                )
                .nest("/jumpstats", jumpstats::router())
                .nest("/records", records::router())
                .nest("/bans", bans::router(cx.clone(), Arc::clone(&cookie_config)));

            cfg_taskdump! {
                let router = router.nest("/taskdump", {
                    taskdump::router()
                        .layer(axum::middleware::from_fn(middleware::auth::client_is_localhost))
                });
            }

            let (in_flight_requests, request_counter) = InFlightRequestsLayer::pair();
            let router = router.nest("/metrics", {
                metrics::router(request_counter)
                    .layer(axum::middleware::from_fn(middleware::auth::client_is_localhost))
            });

            let api_service = ServiceBuilder::new()
                .map_response_body(axum::body::Body::new)
                .layer(in_flight_requests)
                .set_x_request_id(middleware::request_id::make_request_id())
                .propagate_x_request_id()
                .layer(middleware::trace::layer())
                .layer(middleware::cors::layer())
                .layer(middleware::trim_trailing_slash::layer())
                .layer(middleware::catch_panic::layer())
                .service(router.with_state(cx.clone()).into_service())
                .into_make_service_with_connect_info::<SocketAddr>();

            let socket = tokio::net::TcpListener::bind(server_config.socket_addr())
                .await
                .map_err(Error::RunServer)?;

            let addr = socket.local_addr().map_err(Error::RunServer)?;

            info!("Listening on {addr}");

            let (serve_result_tx, mut serve_result_rx) = oneshot::channel();
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            task::spawn(async move {
                let serve_result = axum::serve(socket, api_service)
                    .with_graceful_shutdown(shutdown_rx.map(drop))
                    .await;

                serve_result_tx.send(serve_result)
            });

            cx.spawn("points-daemon", |cancellation_token| {
                cs2kz::points::daemon::run(cx.clone(), cancellation_token)
            });

            select! {
                biased;

                Ok(result) = &mut serve_result_rx => match result {
                    Ok(()) => panic!("server shut down prematurely"),
                    Err(error) => {
                        error!(%error, "failed to run server");
                        return Err(Error::RunServer(error));
                    },
                },

                () = runtime::signal::shutdown() => {},
            }

            let _ = shutdown_tx.send(());

            match future::join!(serve_result_rx, cx.cleanup())
                .timeout(Duration::from_secs(15))
                .await
            {
                Ok((Ok(Ok(())), ())) => Ok(()),
                Ok((Ok(Err(error)), ())) => {
                    error!(%error, "failed to run server");
                    Err(Error::RunServer(error))
                },
                Ok((Err(_), ())) => unreachable!("we never drop the sender"),
                Err(_) => {
                    warn!("server did not shut down within timeout");
                    Ok(())
                },
            }
        })
}
