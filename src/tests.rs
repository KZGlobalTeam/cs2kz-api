use std::fmt;
use std::net::SocketAddrV4;

use sqlx::migrate::Migrator;
use sqlx::MySqlPool;
use url::Url;

mod status;
mod players;
mod maps;
mod servers;
mod gameservers;

static MIGRATOR: Migrator = sqlx::migrate!("./database/migrations");

struct Context {
	addr: SocketAddrV4,
	http_client: reqwest::Client,
	connection_pool: MySqlPool,
}

impl Context {
	/// Constructs a new test context used for making API requests and database queries.
	fn new(addr: SocketAddrV4, connection_pool: MySqlPool) -> Self {
		Self { addr, http_client: reqwest::Client::new(), connection_pool }
	}

	/// Creates a URL that can be used to make API requests.
	///
	/// # Panics
	///
	/// This function will panic if the `path` provided prevents constructing a valid URL.
	fn url(&self, path: impl fmt::Display) -> Url {
		Url::parse(&format!("http://{}{}", self.addr, path)).expect("invalid url")
	}
}
