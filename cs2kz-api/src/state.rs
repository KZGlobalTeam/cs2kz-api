use {
	color_eyre::{eyre::Context, Result},
	sqlx::{mysql::MySqlPoolOptions, MySqlPool},
	std::{fmt::Debug, sync::Arc},
};

pub struct State {
	database: MySqlPool,
}

impl State {
	pub async fn new(database_url: &str) -> Result<Arc<Self>> {
		let database = MySqlPoolOptions::new()
			.connect(database_url)
			.await
			.context("Failed to establish database connection.")?;

		Ok(Arc::new(Self { database }))
	}

	pub const fn database(&self) -> &MySqlPool {
		&self.database
	}
}

impl Debug for State {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("State")
	}
}
