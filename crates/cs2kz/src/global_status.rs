//! Global Status for KZ maps.
//!
//! When maps are submitted for global approval, they will either undergo a public [testing] phase,
//! or be [globalled] right away. At some later point they might be [degloballed] again because the
//! creator requested it, or because the map approval team decided so.
//!
//! [testing]: type@GlobalStatus::InTesting
//! [globalled]: type@GlobalStatus::Global
//! [degloballed]: type@GlobalStatus::NotGlobal

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// The global status of a map.
#[repr(i8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GlobalStatus {
	/// The map is not global.
	NotGlobal = -1,

	/// The map is in a public testing phase.
	InTesting = 0,

	/// The map is global.
	#[default]
	Global = 1,
}

impl GlobalStatus {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::NotGlobal => "not_global",
			Self::InTesting => "in_testing",
			Self::Global => "global",
		}
	}
}

impl Display for GlobalStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::NotGlobal => "Not Global",
			Self::InTesting => "In Testing",
			Self::Global => "Global",
		})
	}
}

impl From<GlobalStatus> for i8 {
	fn from(global_status: GlobalStatus) -> Self {
		global_status as i8
	}
}

impl TryFrom<i8> for GlobalStatus {
	type Error = Error;

	fn try_from(value: i8) -> Result<Self> {
		match value {
			-1 => Ok(Self::NotGlobal),
			0 => Ok(Self::InTesting),
			1 => Ok(Self::Global),
			_ => Err(Error::InvalidGlobalStatus),
		}
	}
}

impl FromStr for GlobalStatus {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<i8>() {
			return Self::try_from(value);
		}

		match value {
			"not_global" | "not global" => Ok(Self::NotGlobal),
			"in_testing" | "in testing" => Ok(Self::InTesting),
			"global" => Ok(Self::Global),
			_ => Err(Error::InvalidGlobalStatus),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::GlobalStatus;

		impl GlobalStatus {
			/// Serialize in a API compatible format.
			pub fn serialize_api<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.api().serialize(serializer)
			}

			/// Serialize this global status as an integer value.
			pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				(*self as i8).serialize(serializer)
			}
		}

		impl Serialize for GlobalStatus {
			/// Uses the [`GlobalStatus::serialize_api()`] method.
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

		use crate::GlobalStatus;

		impl GlobalStatus {
			/// Deserializes the value returned by [`GlobalStatus::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"not_global" | "not global" => Ok(Self::NotGlobal),
					"in_testing" | "in testing" => Ok(Self::InTesting),
					"global" => Ok(Self::Global),
					value => Err(Error::invalid_value(
						U::Str(value),
						&"`not_global` | `in_testing` | `global`",
					)),
				}
			}

			/// Deserializes a global status integer value.
			pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <i8>::deserialize(deserializer)? {
					-1 => Ok(Self::NotGlobal),
					0 => Ok(Self::InTesting),
					1 => Ok(Self::Global),
					value => Err(Error::invalid_value(
						U::Signed(value as i64),
						&"-1, 0, or 1",
					)),
				}
			}
		}

		impl<'de> Deserialize<'de> for GlobalStatus {
			/// Best-effort attempt at deserializing a [`GlobalStatus`] of unknown
			/// format.
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
					I8(i8),
					Str(&'a str),
				}

				match <Helper<'de>>::deserialize(deserializer)? {
					Helper::I8(value) => value.try_into(),
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

	use crate::GlobalStatus;

	impl<DB> Type<DB> for GlobalStatus
	where
		DB: Database,
		i8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<i8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for GlobalStatus
	where
		DB: Database,
		i8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			(*self as i8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for GlobalStatus
	where
		DB: Database,
		i8: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			i8::decode(value).map(Self::try_from)?.map_err(Into::into)
		}
	}
}

#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::schema::AnyOfBuilder;
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::GlobalStatus;

	impl<'s> ToSchema<'s> for GlobalStatus {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"GlobalStatus",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("global".into()))
						.default(Some("global".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("global".into()))
								.enum_values(Some(["not_global", "in_testing", "global"]))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Integer"))
								.schema_type(SchemaType::Integer)
								.example(Some(1.into()))
								.enum_values(Some(-1..=1))
								.build(),
						))
						.build(),
				)
				.into(),
			)
		}
	}

	impl IntoParams for GlobalStatus {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("global_status")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
