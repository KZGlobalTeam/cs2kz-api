use sqlx::types::Json as SqlJson;
use sqlx::MySqlPool;
use tokio::task;
use tracing::error;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer as _;

use super::layer::Consumer;
use crate::logging::{Layer, Log};

pub struct AuditLogs {
	database: MySqlPool,
}

impl AuditLogs {
	pub fn layer<S>(database: MySqlPool) -> impl tracing_subscriber::Layer<S>
	where
		S: tracing::Subscriber + for<'a> LookupSpan<'a>,
	{
		Layer::new(Self { database })
			.with_filter(FilterFn::new(|metadata| metadata.target() == "audit_log"))
	}
}

impl Consumer for AuditLogs {
	fn save_log(&self, Log { level, source, message, fields, .. }: Log) {
		let database = self.database.clone();
		let query = sqlx::query! {
			r#"
			INSERT INTO
			  AuditLogs (level, source, message, fields)
			VALUES
			  (?, ?, ?, ?)
			"#,
			level.as_str(),
			source,
			message,
			SqlJson(fields),
		};

		task::spawn(async move {
			if let Err(error) = query.execute(&database).await {
				error!(%error, "failed to save audit log");
			}
		});
	}
}
