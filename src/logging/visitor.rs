use std::fmt;

use serde::Serialize;
use serde_json::Value as JsonValue;
use tracing::field::{self, Field};

type JsonObject = serde_json::Map<String, JsonValue>;

/// [`tracing`] field visitor for recording information.
#[derive(Debug, Serialize)]
pub struct Visitor {
	pub(super) level: &'static str,
	pub(super) source: String,
	pub(super) message: Option<String>,
	pub(super) fields: JsonObject,

	#[serde(skip)]
	pub(super) is_audit: bool,

	#[serde(skip)]
	pub(super) is_axiom: bool,
}

impl Visitor {
	pub fn new(metadata: &'static tracing::Metadata<'static>) -> Self {
		let level = metadata.level().as_str();
		let source = metadata
			.module_path()
			.map(ToOwned::to_owned)
			.unwrap_or_else(|| String::from("unknown"));

		Self {
			level,
			source,
			message: None,
			fields: Default::default(),
			is_audit: false,
			is_axiom: true,
		}
	}

	fn set_field(&mut self, field: &Field, value: impl Into<JsonValue>) {
		match (field.name(), value.into()) {
			("audit", JsonValue::Bool(is_audit)) => {
				self.is_audit = is_audit;
			}
			("skip_axiom", JsonValue::Bool(skip_axiom)) => {
				self.is_axiom = skip_axiom;
			}
			("message", value) => {
				self.message = Some(value.to_string());
			}
			(field, value) => {
				self.fields.insert(field.to_owned(), value);
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
		match field.name() {
			"audit" => {
				self.is_audit = value;
			}
			"skip_axiom" => {
				self.is_axiom = false;
			}
			_ => {
				self.set_field(field, value);
			}
		}
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		if field.name() == "message" {
			self.message = Some(value.to_owned());
			return;
		}

		self.set_field(field, value);
	}
}
