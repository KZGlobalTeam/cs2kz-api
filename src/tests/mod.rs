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
	fn new(addr: SocketAddrV4, connection_pool: MySqlPool) -> Self {
		Self { addr, http_client: reqwest::Client::new(), connection_pool }
	}

	fn url(&self, path: impl fmt::Display) -> Url {
		Url::parse(&format!("http://{}{}", self.addr, path)).expect("invalid url")
	}
}
