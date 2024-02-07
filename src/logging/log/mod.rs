use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use serde::{Serialize, Serializer};
use tracing::field::{self, Field};
use tracing::{span, Level};

mod value;
pub use value::Value;

/// A tracing visitor that can record span/event fields.
#[derive(Debug, Serialize)]
pub struct Log {
	#[serde(serialize_with = "Log::serialize_level")]
	pub level: &'static Level,
	pub source: Option<&'static str>,

	#[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
	pub fields: HashMap<&'static str, Value>,
}

impl Log {
	pub fn message(&mut self) -> Option<String> {
		if let Value::String(message) = self.fields.remove("message")? {
			Some(message)
		} else {
			panic!("invalid type for message");
		}
	}

	pub fn field(&self, name: &str) -> Option<&Value> {
		self.fields.get(name)
	}

	fn set_field(&mut self, field: &Field, value: impl Into<Value>) {
		self.fields.insert(field.name(), value.into());
	}

	fn serialize_level<S>(level: &Level, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		level.as_str().serialize(serializer)
	}
}

impl From<&tracing::Event<'_>> for Log {
	fn from(event: &tracing::Event<'_>) -> Self {
		let mut log = Self::from(event.metadata());
		event.record(&mut log);
		log
	}
}

impl From<&span::Attributes<'_>> for Log {
	fn from(attributes: &span::Attributes<'_>) -> Self {
		let mut log = Self::from(attributes.metadata());
		attributes.record(&mut log);
		log
	}
}

impl From<&'static tracing::Metadata<'_>> for Log {
	fn from(metadata: &'static tracing::Metadata<'_>) -> Self {
		Self {
			level: metadata.level(),
			source: metadata.module_path(),
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
