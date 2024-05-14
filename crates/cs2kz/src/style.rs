//! The styles available in CS2KZ.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// All official gameplay styles included in the CS2KZ plugin.
#[repr(u8)]
#[non_exhaustive]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Style {
	/// The "normal" style (default).
	#[default]
	Normal = 1,

	/// Perfect automatic bunnyhopping.
	AutoBhop = 2,
}

impl Style {
	/// Checks whether `self` is the default [Normal] style.
	///
	/// [Normal]: Self::Normal
	pub const fn is_normal(&self) -> bool {
		matches!(*self, Self::Normal)
	}

	/// Returns a string representation of this [Style], as accepted by the API.
	pub const fn as_str(&self) -> &'static str {
		match *self {
			Self::Normal => "normal",
			Self::AutoBhop => "auto_bhop",
		}
	}

	/// Returns a short string representation of this [Style], as displayed in-game.
	pub const fn as_str_short(&self) -> &'static str {
		match *self {
			Self::Normal => "NRM",
			Self::AutoBhop => "ABH",
		}
	}
}

impl Display for Style {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(match *self {
			Self::Normal => "Normal",
			Self::AutoBhop => "Auto Bhop",
		})
	}
}

/// Error for parsing a string into a [`Style`].
#[derive(Debug, Clone, Error)]
#[error("unrecognized style `{0}`")]
pub struct UnknownStyle(pub String);

impl FromStr for Style {
	type Err = UnknownStyle;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();

		match s.as_str() {
			"nrm" | "normal" => Ok(Self::Normal),
			"abh" | "auto_bhop" | "auto-bhop" => Ok(Self::AutoBhop),
			_ => Err(UnknownStyle(s)),
		}
	}
}

/// Error for converting a style ID into a [`Style`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid style ID `{0}`")]
pub struct InvalidStyleID(pub u8);

impl TryFrom<u8> for Style {
	type Error = InvalidStyleID;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(Self::Normal),
			2 => Ok(Self::AutoBhop),
			invalid => Err(InvalidStyleID(invalid)),
		}
	}
}

impl From<Style> for u8 {
	#[allow(clippy::as_conversions)]
	fn from(value: Style) -> Self {
		value as u8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

	use super::Style;

	impl Serialize for Style {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_str().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Style {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			#[derive(Deserialize)]
			#[serde(untagged)]
			#[allow(clippy::missing_docs_in_private_items)]
			enum Helper {
				U8(u8),
				Str(String),
			}

			Helper::deserialize(deserializer).and_then(|value| match value {
				Helper::U8(int) => Self::try_from(int).map_err(de::Error::custom),
				Helper::Str(str) => str.parse().map_err(de::Error::custom),
			})
		}
	}
}

/// Method and Trait implementations when depending on [`sqlx`].
#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use sqlx::database::{HasArguments, HasValueRef};
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use super::Style;

	impl<DB> Type<DB> for Style
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for Style
	where
		DB: Database,
		u8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			<u8 as Encode<'q, DB>>::encode_by_ref(&u8::from(*self), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for Style
	where
		DB: Database,
		u8: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			<u8 as Decode<'r, DB>>::decode(value)
				.map(Self::try_from)?
				.map_err(Into::into)
		}
	}
}

/// Method and Trait implementations when depending on [`utoipa`].
#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::schema::AnyOfBuilder;
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::Style;

	impl<'s> ToSchema<'s> for Style {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"Style",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("normal".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("normal".into()))
								.enum_values(Some(["normal", "auto_bhop"]))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("ID"))
								.schema_type(SchemaType::Integer)
								.example(Some(1.into()))
								.enum_values(Some(1..=2))
								.build(),
						))
						.build(),
				)
				.into(),
			)
		}
	}

	impl IntoParams for Style {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.name("style")
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
