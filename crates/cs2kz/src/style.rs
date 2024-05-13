//! An enum for the styles available in CS2KZ.

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// The current gameplay styles in CS2KZ.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Style {
	/// The default style
	#[default]
	Normal = 1,

	/// You can only move backwards
	Backwards = 2,

	/// You can only move sideways
	Sideways = 3,

	/// You can only move half-sideways
	HalfSideways = 4,

	/// You can only use +forward
	WOnly = 5,

	/// Low gravity
	LowGravity = 6,

	/// High gravity
	HighGravity = 7,

	/// No prestrafing allowed
	NoPrestrafe = 8,

	/// You have to hold a negev (lower running speed)
	Negev = 9,

	/// The floor is ice
	Ice = 10,
}

impl Style {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::Normal => "normal",
			Self::Backwards => "backwards",
			Self::Sideways => "sideways",
			Self::HalfSideways => "half_sideways",
			Self::WOnly => "w_only",
			Self::LowGravity => "low_gravity",
			Self::HighGravity => "high_gravity",
			Self::NoPrestrafe => "no_prestrafe",
			Self::Negev => "negev",
			Self::Ice => "ice",
		}
	}
}

impl Display for Style {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Normal => "Normal",
			Self::Backwards => "Backwards",
			Self::Sideways => "Sideways",
			Self::HalfSideways => "Half Sideways",
			Self::WOnly => "W Only",
			Self::LowGravity => "Low Gravity",
			Self::HighGravity => "High Gravity",
			Self::NoPrestrafe => "No Prestrafe",
			Self::Negev => "Negev",
			Self::Ice => "Ice",
		})
	}
}

impl From<Style> for u8 {
	fn from(style: Style) -> Self {
		style as u8
	}
}

impl TryFrom<u8> for Style {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::Normal),
			2 => Ok(Self::Backwards),
			3 => Ok(Self::Sideways),
			4 => Ok(Self::HalfSideways),
			5 => Ok(Self::WOnly),
			6 => Ok(Self::LowGravity),
			7 => Ok(Self::HighGravity),
			8 => Ok(Self::NoPrestrafe),
			9 => Ok(Self::Negev),
			10 => Ok(Self::Ice),
			_ => Err(Error::InvalidStyle),
		}
	}
}

impl FromStr for Style {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<u8>() {
			return Self::try_from(value);
		}

		match value {
			"normal" => Ok(Self::Normal),
			"backwards" => Ok(Self::Backwards),
			"sideways" => Ok(Self::Sideways),
			"half_sideways" => Ok(Self::HalfSideways),
			"w_only" => Ok(Self::WOnly),
			"low_gravity" => Ok(Self::LowGravity),
			"high_gravity" => Ok(Self::HighGravity),
			"no_prestrafe" => Ok(Self::NoPrestrafe),
			"negev" => Ok(Self::Negev),
			"ice" => Ok(Self::Ice),
			_ => Err(Error::InvalidStyle),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::Style;

		impl Style {
			/// Serialize in a API compatible format.
			pub fn serialize_api<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.api().serialize(serializer)
			}

			/// Serialize this style's ID.
			pub fn serialize_id<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				(*self as u8).serialize(serializer)
			}
		}

		impl Serialize for Style {
			/// Uses the [`Style::serialize_api()`] method.
			///
			/// If you need a different format, consider using
			/// `#[serde(serialize_with = "…")]` with one of the other available
			/// `serialize_*` methods.
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.serialize_api(serializer)
			}
		}
	}

	mod de {
		use std::borrow::Cow;

		use serde::de::{Error, Unexpected as U};
		use serde::{Deserialize, Deserializer};

		use crate::Style;

		impl Style {
			/// Deserializes the value returned by [`Style::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"normal" => Ok(Self::Normal),
					"backwards" => Ok(Self::Backwards),
					"sideways" => Ok(Self::Sideways),
					"half_sideways" => Ok(Self::HalfSideways),
					"w_only" => Ok(Self::WOnly),
					"low_gravity" => Ok(Self::LowGravity),
					"high_gravity" => Ok(Self::HighGravity),
					"no_prestrafe" => Ok(Self::NoPrestrafe),
					"negev" => Ok(Self::Negev),
					"ice" => Ok(Self::Ice),
					value => Err(Error::invalid_value(U::Str(value), &"style")),
				}
			}

			/// Deserializes a style ID.
			pub fn deserialize_id<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match u8::deserialize(deserializer)? {
					1 => Ok(Self::Normal),
					2 => Ok(Self::Backwards),
					3 => Ok(Self::Sideways),
					4 => Ok(Self::HalfSideways),
					5 => Ok(Self::WOnly),
					6 => Ok(Self::LowGravity),
					7 => Ok(Self::HighGravity),
					8 => Ok(Self::NoPrestrafe),
					9 => Ok(Self::Negev),
					10 => Ok(Self::Ice),
					value => Err(Error::invalid_value(U::Unsigned(value as u64), &"style ID")),
				}
			}
		}

		impl<'de> Deserialize<'de> for Style {
			/// Best-effort attempt at deserializing a [`Style`] of unknown format.
			///
			/// If you know / expect the specific format, consider using
			/// `#[serde(deserialize_with = "…")]` with one of the `deserialize_*`
			/// methods instead.
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Deserialize)]
				#[serde(untagged)]
				enum Helper<'a> {
					U8(u8),
					Str(Cow<'a, str>),
				}

				match <Helper<'de>>::deserialize(deserializer)? {
					Helper::U8(value) => value.try_into(),
					Helper::Str(value) => value.parse(),
				}
				.map_err(Error::custom)
			}
		}
	}
}

#[cfg(feature = "sqlx")]
mod sqlx_impls {
	use sqlx::database::{HasArguments, HasValueRef};
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use crate::Style;

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
			(*self as u8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for Style
	where
		DB: Database,
		u8: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			u8::decode(value).map(Self::try_from)?.map_err(Into::into)
		}
	}
}

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
								.enum_values(Some([
									"normal",
									"backwards",
									"sideways",
									"half_sideways",
									"w_only",
									"low_gravity",
									"high_gravity",
									"no_prestrafe",
									"negev",
									"ice",
								]))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("ID"))
								.schema_type(SchemaType::Integer)
								.example(Some(1.into()))
								.enum_values(Some(1..=10))
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
