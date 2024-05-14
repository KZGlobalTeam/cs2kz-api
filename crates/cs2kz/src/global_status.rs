//! Global Status for KZ maps.
//!
//! When maps are submitted for global approval, they will either undergo a public [testing] phase,
//! or be [globalled] right away. At some later point they might be [degloballed] again because the
//! creator requested it, or because the map approval team decided so.
//!
//! [testing]: GlobalStatus::InTesting
//! [globalled]: GlobalStatus::Global
//! [degloballed]: GlobalStatus::NotGlobal

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// The global status of a map.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum GlobalStatus {
	/// The map is not global.
	NotGlobal = -1,

	/// The map is in a public testing phase.
	InTesting = 0,

	/// The map is global.
	Global = 1,
}

impl GlobalStatus {
	/// Checks whether `self` is [Global].
	///
	/// [Global]: Self::Global
	pub const fn is_global(&self) -> bool {
		matches!(*self, Self::Global)
	}

	/// Checks whether `self` is [in testing].
	///
	/// [in testing]: Self::InTesting
	pub const fn is_in_testing(&self) -> bool {
		matches!(*self, Self::InTesting)
	}

	/// Returns a string representation of this [GlobalStatus], as accepted by the API.
	pub const fn as_str(&self) -> &'static str {
		match *self {
			Self::NotGlobal => "not_global",
			Self::InTesting => "in_testing",
			Self::Global => "global",
		}
	}
}

impl Display for GlobalStatus {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(match *self {
			Self::NotGlobal => "not global",
			Self::InTesting => "in testing",
			Self::Global => "global",
		})
	}
}

/// Error for parsing a string into a [`GlobalStatus`].
#[derive(Debug, Clone, Error)]
#[error("unknown global status `{0}`")]
pub struct UnknownGlobalStatus(pub String);

impl FromStr for GlobalStatus {
	type Err = UnknownGlobalStatus;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();

		match s.as_str() {
			"not global" | "not_global" => Ok(Self::NotGlobal),
			"in testing" | "in_testing" => Ok(Self::InTesting),
			"global" => Ok(Self::Global),
			_ => Err(UnknownGlobalStatus(s)),
		}
	}
}

/// Error for converting an integer to a [`GlobalStatus`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid global status `{0}`")]
pub struct InvalidGlobalStatus(pub i8);

impl TryFrom<i8> for GlobalStatus {
	type Error = InvalidGlobalStatus;

	fn try_from(value: i8) -> Result<Self, Self::Error> {
		match value {
			-1 => Ok(Self::NotGlobal),
			0 => Ok(Self::InTesting),
			1 => Ok(Self::Global),
			invalid => Err(InvalidGlobalStatus(invalid)),
		}
	}
}

impl From<GlobalStatus> for i8 {
	#[allow(clippy::as_conversions)]
	fn from(value: GlobalStatus) -> Self {
		value as i8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{de, Deserialize, Deserializer};

	use super::GlobalStatus;

	impl<'de> Deserialize<'de> for GlobalStatus {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			#[derive(Deserialize)]
			#[serde(untagged)]
			#[allow(clippy::missing_docs_in_private_items)]
			enum Helper {
				I8(i8),
				Str(String),
			}

			Helper::deserialize(deserializer).and_then(|value| match value {
				Helper::I8(int) => Self::try_from(int).map_err(de::Error::custom),
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

	use super::GlobalStatus;

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
			<i8 as Encode<'q, DB>>::encode_by_ref(&i8::from(*self), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for GlobalStatus
	where
		DB: Database,
		i8: Decode<'r, DB>,
	{
		fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
			<i8 as Decode<'r, DB>>::decode(value)
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
