use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use tracing::field::{self, Field};
use tracing::Level;

mod value;
use value::Value;

/// A tracing visitor that can record span/event fields.
pub struct Log {
	pub level: &'static Level,
	pub target: &'static str,
	pub source: Option<&'static str>,
	pub message: Option<String>,
	pub fields: HashMap<&'static str, Value>,
}

impl Log {
	fn set_field(&mut self, field: &Field, value: impl Into<Value>) {
		self.fields.insert(field.name(), value.into());
	}
}

impl From<&tracing::Event<'_>> for Log {
	fn from(event: &tracing::Event<'_>) -> Self {
		let mut log = Self::from(event.metadata());
		event.record(&mut log);
		log
	}
}

impl From<&'static tracing::Metadata<'_>> for Log {
	fn from(metadata: &'static tracing::Metadata<'_>) -> Self {
		Self {
			level: metadata.level(),
			target: metadata.target(),
			source: metadata.module_path(),
			message: None,
			fields: HashMap::new(),
		}
	}
}

impl field::Visit for Log {
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
		self.set_field(field, value);
	}

	fn record_u128(&mut self, field: &Field, value: u128) {
		self.set_field(field, value);
	}

	fn record_bool(&mut self, field: &Field, value: bool) {
		self.set_field(field, value);
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		self.set_field(field, value);
	}

	fn record_error(&mut self, field: &Field, value: &(dyn Error + 'static)) {
		self.set_field(field, value);
	}

	fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
		self.set_field(field, value);
	}
}
