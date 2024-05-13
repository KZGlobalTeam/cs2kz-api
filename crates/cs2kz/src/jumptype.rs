//! The different types of jumps in CS2KZ.

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// All the jump types registered as jumpstats in KZ.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JumpType {
	/// The standard longjump.
	LongJump = 1,

	/// A single bhop.
	SingleBhop = 2,

	/// Multiple chained bhops.
	MultiBhop = 3,

	/// Walking off an elevated surface and bhopping.
	WeirdJump = 4,

	/// Jumping off a ladder.
	LadderJump = 5,

	/// Jumping from the ground below a ladder.
	LadderHop = 6,
}

impl JumpType {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::LongJump => "longjump",
			Self::SingleBhop => "single_bhop",
			Self::MultiBhop => "multi_bhop",
			Self::WeirdJump => "weirdjump",
			Self::LadderJump => "ladderjump",
			Self::LadderHop => "ladderhop",
		}
	}

	/// A shortened form of the jump type's name.
	#[inline]
	pub const fn short(&self) -> &'static str {
		match self {
			Self::LongJump => "LJ",
			Self::SingleBhop => "BH",
			Self::MultiBhop => "MBH",
			Self::WeirdJump => "WJ",
			Self::LadderJump => "LAJ",
			Self::LadderHop => "LAH",
		}
	}
}

impl Display for JumpType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.short())
	}
}

impl From<JumpType> for u8 {
	fn from(jump_type: JumpType) -> Self {
		jump_type as u8
	}
}

impl TryFrom<u8> for JumpType {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::LongJump),
			2 => Ok(Self::SingleBhop),
			3 => Ok(Self::MultiBhop),
			4 => Ok(Self::WeirdJump),
			5 => Ok(Self::LadderJump),
			6 => Ok(Self::LadderHop),
			_ => Err(Error::InvalidJumpType),
		}
	}
}

impl FromStr for JumpType {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<u8>() {
			return Self::try_from(value);
		}

		match value {
			"longjump" | "LJ" => Ok(Self::LongJump),
			"single_bhop" | "BH" => Ok(Self::SingleBhop),
			"multi_bhop" | "MBH" => Ok(Self::MultiBhop),
			"weirdjump" | "WJ" => Ok(Self::WeirdJump),
			"ladderjump" | "LAJ" => Ok(Self::LadderJump),
			"ladderhop" | "LAH" => Ok(Self::LadderHop),
			_ => Err(Error::InvalidJumpType),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::JumpType;

		impl JumpType {
			/// Serialize in a API compatible format.
			pub fn serialize_api<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.api().serialize(serializer)
			}

			/// Serialize as an integer.
			pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				(*self as u8).serialize(serializer)
			}
		}

		impl Serialize for JumpType {
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

		use crate::JumpType;

		impl JumpType {
			/// Deserializes the value returned by [`JumpType::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"longjump" | "LJ" => Ok(Self::LongJump),
					"single_bhop" | "BH" => Ok(Self::SingleBhop),
					"multi_bhop" | "MBH" => Ok(Self::MultiBhop),
					"weirdjump" | "WJ" => Ok(Self::WeirdJump),
					"ladderjump" | "LAJ" => Ok(Self::LadderJump),
					"ladderhop" | "LAH" => Ok(Self::LadderHop),
					value => Err(Error::invalid_value(U::Str(value), &"jump type")),
				}
			}

			/// Deserializes from an integer.
			pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <u8>::deserialize(deserializer)? {
					1 => Ok(Self::LongJump),
					2 => Ok(Self::SingleBhop),
					3 => Ok(Self::MultiBhop),
					4 => Ok(Self::WeirdJump),
					5 => Ok(Self::LadderJump),
					6 => Ok(Self::LadderHop),
					value => Err(Error::invalid_value(
						U::Unsigned(value as u64),
						&"value between 1 and 6",
					)),
				}
			}
		}

		impl<'de> Deserialize<'de> for JumpType {
			/// Best-effort attempt at deserializing a [`JumpType`] of unknown format.
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

	use crate::JumpType;

	impl<DB> Type<DB> for JumpType
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for JumpType
	where
		DB: Database,
		u8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			(*self as u8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for JumpType
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

	use crate::JumpType;

	impl<'s> ToSchema<'s> for JumpType {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"JumpType",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("longjump".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("longjump".into()))
								.enum_values(Some([
									"longjump",
									"single_bhop",
									"multi_bhop",
									"weirdjump",
									"ladderjump",
									"ladderhop",
								]))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("ID"))
								.schema_type(SchemaType::Integer)
								.example(Some(1.into()))
								.enum_values(Some(1..=6))
								.build(),
						))
						.build(),
				)
				.into(),
			)
		}
	}

	impl IntoParams for JumpType {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("jump_type")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
