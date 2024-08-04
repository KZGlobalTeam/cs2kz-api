//! This module contains the [`ExtensionMembers`] struct.
//!
//! It represents arbitrary additional fields to add to an error response.

use std::fmt;

use serde::Serialize;

/// [RFC 9457] [Extension Members].
///
/// [RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
/// [Extension Members]: https://www.rfc-editor.org/rfc/rfc9457.html#name-extension-members
#[derive(Serialize)]
#[serde(transparent)]
pub struct ExtensionMembers
{
	/// JSON object that encodes the extra values.
	#[serde(flatten)]
	obj: serde_json::Map<String, serde_json::Value>,
}

impl ExtensionMembers
{
	/// Creates a new [`ExtensionMembers`].
	pub fn new() -> Self
	{
		Self { obj: Default::default() }
	}

	/// Adds an extension member.
	///
	/// # Panics
	///
	/// This function will panic if `value` cannot be serialized into JSON.
	pub fn add<V>(&mut self, name: impl Into<String>, value: &V)
	where
		V: Serialize + ?Sized,
	{
		let value = serde_json::to_value(value).expect("value should be valid JSON");

		self.obj.insert(name.into(), value);
	}
}

impl fmt::Debug for ExtensionMembers
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Debug::fmt(&self.obj, f)
	}
}
