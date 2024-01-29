use std::net::SocketAddr;
use std::{fmt, io};

use axum::http::{header, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use itertools::Itertools;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::debug;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use self::auth::openapi::Security;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use cs2kz_api_macros::test;

mod error;
pub use error::{Error, Result};

pub mod config;
pub use config::Config;

mod state;
pub use state::State;

/// Convenience alias for extracting [`State`] in handlers.
pub type AppState = axum::extract::State<&'static crate::State>;

mod database;
mod middleware;
mod params;
mod query;
mod responses;
mod sqlx;
mod status;
mod steam;
mod util;

mod players;
mod maps;
mod servers;
mod bans;
mod auth;

#[derive(OpenApi)]
#[rustfmt::skip]
#[openapi(
  info(
    title = "CS2KZ API",
    license(
      name = "GPL-3.0",
      url = "https://www.gnu.org/licenses/gpl-3.0",
    ),
  ),
  modifiers(&Security),
  components(
    schemas(
      cs2kz::SteamID,
      cs2kz::Mode,
      cs2kz::Style,
      cs2kz::Jumpstat,
      cs2kz::Tier,
      cs2kz::PlayerIdentifier,
      cs2kz::MapIdentifier,
      cs2kz::ServerIdentifier,

      params::Limit,
      params::Offset,

      database::RankedStatus,
      database::GlobalStatus,

      auth::Role,
      auth::RoleFlags,
      auth::servers::RefreshToken,

      players::models::Player,
      players::models::NewPlayer,
      players::models::Admin,

      maps::models::KZMap,
      maps::models::Course,
      maps::models::Filter,
      maps::models::NewMap,
      maps::models::NewCourse,
      maps::models::NewFilter,
      maps::models::MapUpdate,
      maps::models::CourseUpdate,
      maps::models::FilterUpdate,

      servers::models::Server,
      servers::models::NewServer,
      servers::models::CreatedServer,
      servers::models::ServerUpdate,

      bans::models::Ban,
      bans::models::BannedPlayer,
      bans::models::Unban,
      bans::models::NewBan,
      bans::models::CreatedBan,
      bans::models::BanUpdate,
      bans::models::NewUnban,
      bans::models::CreatedUnban,
    ),
  ),
  paths(
    status::hello_world,

    players::routes::get_many::get_many,
    players::routes::create::create,
    players::routes::get_admins::get_admins,
    players::routes::get_single::get_single,
    players::routes::get_roles::get_roles,
    players::routes::update_roles::update_roles,

    maps::routes::get_many::get_many,
    maps::routes::create::create,
    maps::routes::get_single::get_single,
    maps::routes::update::update,

    servers::routes::get_many::get_many,
    servers::routes::create::create,
    servers::routes::get_single::get_single,
    servers::routes::update::update,
    servers::routes::replace_key::replace_key,
    servers::routes::delete_key::delete_key,

    bans::routes::get_many::get_many,
    bans::routes::create::create,
    bans::routes::get_single::get_single,
    bans::routes::update::update,
    bans::routes::unban::unban,

    auth::routes::login::login,
    auth::routes::logout::logout,
    auth::steam::routes::callback::callback,
    auth::servers::routes::refresh_key::refresh_key,
  ),
)]
pub struct API {
	tcp_listener: TcpListener,
	state: State,
}

impl API {
	/// Creates a new API instance with the given `config`.
	///
	/// See [`API::run()`] for starting the server.
	#[tracing::instrument]
	pub async fn new(config: Config) -> Result<Self> {
		let tcp_listener = TcpListener::bind(config.socket_addr()).await?;
		let local_addr = tcp_listener.local_addr()?;

		debug!(%local_addr, "Initialized TCP socket");

		let state = State::new(config).await?;

		debug!("Initialized API state");

		Ok(Self { tcp_listener, state })
	}

	/// Runs the [axum] server for the API.
	#[tracing::instrument]
	pub async fn run(self) {
		let state: &'static _ = Box::leak(Box::new(self.state));

		// FIXME(AlphaKeks)
		let cors = CorsLayer::new()
			.allow_origin(if state.in_dev() {
				"http://127.0.0.1".parse::<HeaderValue>().unwrap()
			} else {
				"https://dashboard.cs2.kz".parse::<HeaderValue>().unwrap()
			})
			.allow_credentials(true)
			.allow_methods([
				Method::GET,
				Method::POST,
				Method::PUT,
				Method::PATCH,
				Method::DELETE,
			])
			.allow_headers([header::CONTENT_TYPE, header::COOKIE]);

		let service = Router::new()
			.route("/", get(status::hello_world))
			.with_state(state)
			.nest("/players", players::router(state))
			.nest("/maps", maps::router(state))
			.nest("/servers", servers::router(state))
			.nest("/bans", bans::router(state))
			.nest("/auth", auth::router(state))
			.layer(cors)
			.layer(axum::middleware::from_fn(middleware::logging::layer))
			.merge(Self::swagger_ui())
			.into_make_service();

		audit!("starting axum server");

		axum::serve(self.tcp_listener, service)
			.await
			.expect("failed to run axum");
	}

	/// Returns the local socket address for the underlying TCP server.
	pub fn local_addr(&self) -> io::Result<SocketAddr> {
		self.tcp_listener.local_addr()
	}

	/// Returns an iterator over all the routes registered in the OpenAPI spec.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_iter().map(|(uri, path)| {
			let methods = path
				.operations
				.into_keys()
				.map(|method| format!("{method:?}").to_uppercase())
				.collect_vec()
				.join(", ");

			format!("`{uri}` [{methods}]")
		})
	}

	/// Returns a router for hosting a SwaggerUI web page and the OpenAPI spec as a JSON file.
	pub fn swagger_ui() -> SwaggerUi {
		SwaggerUi::new("/docs/swagger-ui").url("/docs/open-api.json", Self::openapi())
	}

	/// Returns a pretty-printed version of the OpenAPI spec in JSON.
	pub fn spec() -> String {
		Self::openapi()
			.to_pretty_json()
			.expect("Failed to format API spec as JSON.")
	}
}

impl fmt::Debug for API {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("CS2KZ API")
			.field("state", &self.state)
			.finish()
	}
}

/// Logs a message with `audit = true`.
///
/// This will cause the log to be saved in the database.
#[macro_export]
macro_rules! audit {
	($level:ident, $message:literal $(,$($fields:tt)*)?) => {
		::tracing::$level!(audit = true, $($($fields)*,)? $message)
	};

	($message:literal $(,$($fields:tt)*)?) => {
		audit!(trace, $message $(,$($fields)*)?)
	};
}

#[cfg(test)]
mod test_setup {
	use tracing_subscriber::EnvFilter;

	#[ctor::ctor]
	fn test_setup() {
		color_eyre::install().unwrap();
		dotenvy::dotenv().unwrap();
		tracing_subscriber::fmt()
			.with_env_filter(EnvFilter::from_default_env())
			.init();
	}
}
