use std::collections::HashMap;
use std::fmt::Debug;

use serde::Serialize;
use serde_json::Value as JsonValue;
use tracing::field::{self, Field};

type JsonObject = HashMap<&'static str, String>;

/// [`tracing`] field visitor for recording information.
#[derive(Debug, Serialize)]
pub struct Log {
	pub(super) level: &'static str,
	pub(super) target: &'static str,
	pub(super) source: &'static str,
	pub(super) message: Option<String>,
	pub(super) fields: JsonObject,
}

impl From<&'static tracing::Metadata<'static>> for Log {
	fn from(metadata: &'static tracing::Metadata<'static>) -> Self {
		let level = metadata.level().as_str();
		let target = metadata.target();
		let source = metadata.module_path().unwrap_or("unknown");

		Self { level, target, source, message: None, fields: Default::default() }
	}
}

impl Log {
	fn set_field(&mut self, field: &Field, value: impl Into<JsonValue>) {
		match (field.name(), value.into()) {
			("message", value) => {
				self.message = Some(stringify_json_value(value));
			}
			(field, value) => match self.fields.get_mut(field) {
				None => {
					let _ = self.fields.insert(field, stringify_json_value(value));
				}
				Some(old) => {
					*old = stringify_json_value(value);
				}
			},
		}
	}
}

fn stringify_json_value(value: JsonValue) -> String {
	match value {
		JsonValue::Null => String::from("null"),
		JsonValue::Bool(x) => x.to_string(),
		JsonValue::Number(x) => x.to_string(),
		JsonValue::String(x) => x,
		val @ (JsonValue::Array(_) | JsonValue::Object(_)) => format!("{val}"),
	}
}

impl field::Visit for Log {
	fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
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
