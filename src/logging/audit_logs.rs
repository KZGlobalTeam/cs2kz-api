use crossbeam::queue::SegQueue;
use sqlx::types::Json as SqlJson;
use sqlx::{MySqlConnection, QueryBuilder};
use tokio::sync::Mutex;
use tokio::task;
use tracing::error;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer as _;

use crate::logging::layer::Consumer;
use crate::logging::{Layer, Log};

pub struct AuditLogs {
	database: Mutex<MySqlConnection>,
	queue: SegQueue<Log>,
}

impl AuditLogs {
	pub fn layer<S>(database: MySqlConnection) -> impl tracing_subscriber::Layer<S>
	where
		S: tracing::Subscriber + for<'a> LookupSpan<'a>,
	{
		let database = Mutex::new(database);
		let queue = SegQueue::new();

		Layer::new(Self { database, queue })
			.with_filter(FilterFn::new(|metadata| metadata.target() == "audit_log"))
	}
}

impl Consumer for AuditLogs {
	fn save_log(&'static self, mut log: Log) {
		let Ok(mut database) = self.database.try_lock() else {
			self.queue.push(log);
			return;
		};

		if self.queue.is_empty() {
			let message = log.message();

			let query = sqlx::query! {
				r#"
				INSERT INTO
				  AuditLogs (level, source, message, fields)
				VALUES
				  (?, ?, ?, ?)
				"#,
				log.level.as_str(),
				log.source,
				message,
				SqlJson(log.fields),
			};

			task::spawn(async move {
				if let Err(error) = query.execute(&mut *database).await {
					error!(%error, "failed to save audit log");
				}
			});
		} else {
			let mut logs = vec![log];

			while let Some(log) = self.queue.pop() {
				logs.push(log);
			}

			let mut query =
				QueryBuilder::new("INSERT INTO AuditLogs (level, source, message, fields)");

			query.push_values(logs, |mut query, mut log| {
				query
					.push_bind(log.level.as_str())
					.push_bind(log.source)
					.push_bind(log.message())
					.push_bind(SqlJson(log.fields));
			});

			task::spawn(async move {
				if let Err(error) = query.build().execute(&mut *database).await {
					error!(%error, "failed to save audit logs");
				}
			});
		}
	}
}
