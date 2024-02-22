use crossbeam::queue::SegQueue;
use sqlx::types::Json;
use sqlx::{MySqlConnection, QueryBuilder};
use tokio::sync::Mutex;
use tokio::task;
use tracing::error;
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer as _;

use crate::logging::layer::ConsumeLog;
use crate::logging::{Layer, Log};

/// Log layer for "important" logs that need to be saved in the database.
pub struct AuditLogs {
	/// This layer only has access to a single connection, so we don't overload the database
	/// with log queries. Because logging is synchronous, we use a queue for backpressure.
	database: Mutex<MySqlConnection>,

	/// Queue for keeping logs while the database connection is unavailable.
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

impl ConsumeLog for AuditLogs {
	// Implementation details:
	//
	// Since we only have a single database connection available to us, only one call to
	// `consume_log` can insert logs into the database at any given time.
	// Any other tasks / threads that call `consume_log` during this time will insert their
	// logs into the queue and return immediately.
	// Whoever has the lock is responsible for emptying the queue and inserting all available
	// logs into the database.
	fn consume_log(&'static self, mut log: Log) {
		let Ok(mut database) = self.database.try_lock() else {
			self.queue.push(log);
			return;
		};

		if self.queue.is_empty() {
			let message = log.message();

			let query = sqlx::query! {
				r#"
				INSERT INTO
				  AuditLogs (`level`, source, message, `fields`)
				VALUES
				  (?, ?, ?, ?)
				"#,
				log.level.as_str(),
				log.source,
				message,
				Json(log.fields),
			};

			task::spawn(async move {
				if let Err(error) = query.execute(&mut *database).await {
					error!(%error, "failed to save audit log");
				}
			});
		} else {
			let mut logs = Vec::with_capacity(self.queue.len() + 1);

			logs.push(log);

			while let Some(log) = self.queue.pop() {
				logs.push(log);
			}

			let mut query =
				QueryBuilder::new("INSERT INTO AuditLogs (`level`, source, message, `fields`)");

			query.push_values(logs, |mut query, mut log| {
				query
					.push_bind(log.level.as_str())
					.push_bind(log.source)
					.push_bind(log.message())
					.push_bind(Json(log.fields));
			});

			task::spawn(async move {
				if let Err(error) = query.build().execute(&mut *database).await {
					error!(%error, "failed to save audit logs");
				}
			});
		}
	}
}
