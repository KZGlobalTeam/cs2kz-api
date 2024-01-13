use std::fmt;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use sqlx::{Connection, MySqlConnection};
use tokio::sync::Mutex;
use tokio::task;
use tracing::error;
use tracing::field::{self, Field};
use tracing_subscriber::layer;

/// Custom [`tracing_subscriber`] log layer for capturing important logs.
pub struct Layer {
	/// Connection to the database into which the logs should be inserted.
	connection: Arc<Mutex<MySqlConnection>>,
}

impl Layer {
	pub async fn new(database_url: &str) -> sqlx::Result<Self> {
		let connection = MySqlConnection::connect(database_url)
			.await
			.map(Mutex::new)
			.map(Arc::new)?;

		Ok(Self { connection })
	}
}

impl<S> tracing_subscriber::Layer<S> for Layer
where
	S: tracing::Subscriber,
{
	fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
		if !event.fields().any(|field| field.name() == "audit") {
			return;
		}

		let location = event.metadata().module_path();
		let connection = Arc::clone(&self.connection);
		let mut visitor = Visitor::new(location);

		visitor.visit(event);

		task::spawn(async move {
			visitor.save(connection).await;
		});
	}
}

struct Visitor {
	location: String,
	message: String,
	data: JsonValue,
}

impl Visitor {
	fn new(location: Option<impl Into<String>>) -> Self {
		Self {
			location: location
				.map(Into::into)
				.unwrap_or_else(|| String::from("unknown")),
			message: String::new(),
			data: JsonValue::Null,
		}
	}

	fn visit(&mut self, event: &tracing::Event<'_>) {
		event.record(self);
	}

	async fn save(self, connection: Arc<Mutex<MySqlConnection>>) {
		let query = sqlx::query! {
			r#"
			INSERT INTO
			  AuditLogs (location, message, fields)
			VALUES
			  (?, ?, ?)
			"#,
			self.location,
			self.message,
			self.data,
		};

		let mut connection = connection.lock().await;

		if let Err(err) = query.execute(&mut *connection).await {
			error!(%err, "failed to save audit logs");
		}
	}

	fn set_field(&mut self, field: &Field, value: impl Into<JsonValue>) {
		match field.name() {
			"audit" => {}
			"message" => {
				self.message = value.into().to_string();
			}
			name => {
				self.data[name] = value.into();
			}
		}
	}
}

impl field::Visit for Visitor {
	fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
		self.set_field(field, format!("{value:?}"));
	}

	fn record_f64(&mut self, field: &Field, value: f64) {
		self.set_field(field, value);
	}

	fn record_i64(&mut self, field: &Field, value: i64) {
		self.set_field(field, value);
	}

	fn record_u64(&mut self, field: &Field, value: u64) {
		self.set_field(field, value);
	}

	fn record_i128(&mut self, field: &Field, value: i128) {
		if let Ok(int) = i64::try_from(value) {
			self.set_field(field, int);
		} else {
			self.set_field(field, value.to_string());
		}
	}

	fn record_u128(&mut self, field: &Field, value: u128) {
		if let Ok(int) = i64::try_from(value) {
			self.set_field(field, int);
		} else {
			self.set_field(field, value.to_string());
		}
	}

	fn record_bool(&mut self, field: &Field, value: bool) {
		self.set_field(field, value);
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		self.set_field(field, value);
	}
}
