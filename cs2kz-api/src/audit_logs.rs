//! Audit Logs.

use std::fmt;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use sqlx::MySqlConnection;
use tokio::sync::Mutex;
use tokio::task;
use tracing::field::{self, Field};
use tracing::Subscriber;
use tracing_subscriber::layer;

/// Custom [`tracing-subscriber`] layer for saving critical logs to a database.
#[allow(missing_debug_implementations)]
pub struct AuditLayer {
	database: Arc<Mutex<MySqlConnection>>,
}

impl AuditLayer {
	/// Constructs a new [`AuditLayer`].
	pub fn new(database: MySqlConnection) -> Self {
		Self { database: Arc::new(Mutex::new(database)) }
	}
}

impl<S: Subscriber> tracing_subscriber::Layer<S> for AuditLayer {
	fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
		if !event.fields().any(|field| field.name() == "audit") {
			return;
		}

		let location = event.metadata().module_path();
		let database = Arc::clone(&self.database);

		task::spawn(Visitor::new(location).visit(event).save(database));
	}
}

struct Visitor<'a> {
	location: &'a str,
	message: String,
	data: JsonValue,
}

impl Visitor<'_> {
	fn new(location: Option<&str>) -> Visitor<'_> {
		Visitor {
			location: location.unwrap_or("unknown"),
			message: String::new(),
			data: JsonValue::Null,
		}
	}

	fn visit(mut self, event: &tracing::Event<'_>) -> Self {
		event.record(&mut self);
		self
	}

	async fn save(self, database: Arc<Mutex<MySqlConnection>>) {
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

		let mut database = database.lock().await;

		if let Err(_error) = query.execute(&mut *database).await {
			todo!();
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

impl field::Visit for Visitor<'_> {
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
