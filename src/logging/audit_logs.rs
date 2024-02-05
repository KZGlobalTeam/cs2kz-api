use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder};
use tokio::task;

use super::{layer, Log};

#[derive(Clone)]
pub struct AuditLogs {
	database: MySqlPool,
}

impl AuditLogs {
	pub async fn new(database_url: &str) -> sqlx::Result<Self> {
		let database = MySqlPool::connect(database_url).await?;

		Ok(Self { database })
	}
}

impl layer::Consumer for AuditLogs {
	fn is_interested_in(metadata: &tracing::Metadata<'_>) -> bool {
		metadata.target() == "audit_log"
	}

	fn would_consume(_: &Log) -> bool {
		true
	}

	fn consume(&self, logs: Vec<Log>) {
		let this = self.clone();
		let mut query = QueryBuilder::new("INSERT INTO AuditLogs (level, source, message, fields)");

		query.push_values(logs, |mut query, log| {
			query
				.push_bind(log.level)
				.push_bind(log.source)
				.push_bind(log.message)
				.push_bind(Json(log.fields));
		});

		task::spawn(async move {
			if let Err(error) = query.build().execute(&this.database).await {
				eprintln!("failed to store audit logs: {error}");
			}
		});
	}
}
