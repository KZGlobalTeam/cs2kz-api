//! An enum for the modes available in CS2KZ.

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// The two modes that currently exist in CS2KZ.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
	/// The default CS2 movement experience.
	#[default]
	Vanilla = 1,

	/// Modified movement settings for an enhanced KZ experience.
	Classic = 2,
}

impl Mode {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::Vanilla => "vanilla",
			Self::Classic => "classic",
		}
	}

	/// A shortened form of the mode's name.
	#[inline]
	pub const fn short(&self) -> &'static str {
		match self {
			Self::Vanilla => "VNL",
			Self::Classic => "CKZ",
		}
	}
}

impl Display for Mode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl From<Mode> for u8 {
	fn from(mode: Mode) -> Self {
		mode as u8
	}
}

impl TryFrom<u8> for Mode {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::Vanilla),
			2 => Ok(Self::Classic),
			_ => Err(Error::InvalidMode),
		}
	}
}

impl FromStr for Mode {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<u8>() {
			return Self::try_from(value);
		}

		match value {
			"vanilla" | "Vanilla" | "vnl" | "VNL" => Ok(Self::Vanilla),
			"classic" | "Classic" | "ckz" | "CKZ" => Ok(Self::Classic),
			_ => Err(Error::InvalidMode),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::Mode;

		impl Mode {
			/// Serialize in a API compatible format.
			pub fn serialize_api<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.api().serialize(serializer)
			}

			/// Serialize the short name of this mode.
			pub fn serialize_short<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.short().serialize(serializer)
			}

			/// Serialize this mode's ID.
			pub fn serialize_id<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				(*self as u8).serialize(serializer)
			}
		}

		impl Serialize for Mode {
			/// Uses the [`Mode::serialize_api()`] method.
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
		use serde::de::{Error, Unexpected as U};
		use serde::{Deserialize, Deserializer};

		use crate::Mode;

		impl Mode {
			/// Deserializes the value returned by [`Mode::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"vanilla" | "Vanilla" => Ok(Self::Vanilla),
					"classic" | "Classic" => Ok(Self::Classic),
					value => Err(Error::invalid_value(
						U::Str(value),
						&"`vanilla` or `classic`",
					)),
				}
			}

			/// Deserializes the value returned by [`Mode::short()`].
			pub fn deserialize_short<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"vnl" | "VNL" => Ok(Self::Vanilla),
					"ckz" | "CKZ" => Ok(Self::Classic),
					value => Err(Error::invalid_value(U::Str(value), &"`vnl` or `ckz`")),
				}
			}

			/// Deserializes a mode ID.
			pub fn deserialize_id<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <u8>::deserialize(deserializer)? {
					1 => Ok(Self::Vanilla),
					2 => Ok(Self::Classic),
					value => Err(Error::invalid_value(U::Unsigned(value as u64), &"1 or 2")),
				}
			}
		}

		impl<'de> Deserialize<'de> for Mode {
			/// Best-effort attempt at deserializing a [`Mode`] of unknown format.
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
					Str(&'a str),
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

	use crate::Mode;

	impl<DB> Type<DB> for Mode
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for Mode
	where
		DB: Database,
		u8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			(*self as u8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for Mode
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

	use crate::Mode;

	impl<'s> ToSchema<'s> for Mode {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"Mode",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("classic".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("classic".into()))
								.enum_values(Some(["vanilla", "classic"]))
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

	impl IntoParams for Mode {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("mode")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
