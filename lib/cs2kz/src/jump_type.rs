//! Jumpstat types.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// All the different kinds of jumpstats.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum JumpType
{
	/// LJ
	LongJump = 1,

	/// BH
	Bhop = 2,

	/// MBH
	MultiBhop = 3,

	/// WJ
	WeirdJump = 4,

	/// LAJ
	LadderJump = 5,

	/// LAH
	Ladderhop = 6,

	/// JB
	Jumpbug = 7,
}

impl JumpType
{
	/// Returns a string representation of this [JumpType], as accepted by the
	/// API.
	pub const fn as_str(&self) -> &'static str
	{
		match *self {
			Self::LongJump => "longjump",
			Self::Bhop => "bhop",
			Self::MultiBhop => "multi_bhop",
			Self::WeirdJump => "weird_jump",
			Self::LadderJump => "ladder_jump",
			Self::Ladderhop => "ladder_hop",
			Self::Jumpbug => "jump_bug",
		}
	}
}

impl Display for JumpType
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
	{
		f.write_str(match *self {
			Self::LongJump => "LongJump",
			Self::Bhop => "Bhop",
			Self::MultiBhop => "MultiBhop",
			Self::WeirdJump => "WeirdJump",
			Self::LadderJump => "LadderJump",
			Self::Ladderhop => "Ladderhop",
			Self::Jumpbug => "Jumpbug",
		})
	}
}

/// Error for parsing a string into a [`JumpType`].
#[derive(Debug, Clone, Error)]
#[error("unrecognized jump type `{0}`")]
pub struct UnknownJumpType(pub String);

impl FromStr for JumpType
{
	type Err = UnknownJumpType;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		let s = s.to_lowercase();

		match s.as_str() {
			"lj" | "longjump" => Ok(Self::LongJump),
			"bh" | "bhop" => Ok(Self::Bhop),
			"mbh" | "multi_bhop" => Ok(Self::MultiBhop),
			"wj" | "weird_jump" => Ok(Self::WeirdJump),
			"laj" | "ladder_jump" => Ok(Self::LadderJump),
			"lah" | "ladder_hop" => Ok(Self::Ladderhop),
			"jb" | "jump_bug" => Ok(Self::Jumpbug),
			_ => Err(UnknownJumpType(s)),
		}
	}
}

/// Error for parsing an integer into a [`JumpType`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid jump type `{0}`")]
pub struct InvalidJumpType(pub u8);

impl TryFrom<u8> for JumpType
{
	type Error = InvalidJumpType;

	fn try_from(value: u8) -> Result<Self, Self::Error>
	{
		match value {
			1 => Ok(Self::LongJump),
			2 => Ok(Self::Bhop),
			3 => Ok(Self::MultiBhop),
			4 => Ok(Self::WeirdJump),
			5 => Ok(Self::LadderJump),
			6 => Ok(Self::Ladderhop),
			7 => Ok(Self::Jumpbug),
			invalid => Err(InvalidJumpType(invalid)),
		}
	}
}

impl From<JumpType> for u8
{
	#[expect(clippy::as_conversions, reason = "casts are required to turn enums into integers")]
	fn from(value: JumpType) -> Self
	{
		value as u8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls
{
	use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

	use super::JumpType;

	impl Serialize for JumpType
	{
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_str().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for JumpType
	{
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: Deserializer<'de>,
		{
			#[derive(Deserialize)]
			#[serde(untagged)]
			#[allow(clippy::missing_docs_in_private_items)]
			enum Helper
			{
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
mod sqlx_impls
{
	use sqlx::encode::IsNull;
	use sqlx::error::BoxDynError;
	use sqlx::{Database, Decode, Encode, Type};

	use super::JumpType;

	impl<DB> Type<DB> for JumpType
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo
		{
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for JumpType
	where
		DB: Database,
		u8: Encode<'q, DB>,
	{
		fn encode_by_ref(
			&self,
			buf: &mut <DB as Database>::ArgumentBuffer<'q>,
		) -> Result<IsNull, sqlx::error::BoxDynError>
		{
			<u8 as Encode<'q, DB>>::encode_by_ref(&u8::from(*self), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for JumpType
	where
		DB: Database,
		u8: Decode<'r, DB>,
	{
		fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, BoxDynError>
		{
			<u8 as Decode<'r, DB>>::decode(value)
				.map(Self::try_from)?
				.map_err(Into::into)
		}
	}
}

/// Method and Trait implementations when depending on [`utoipa`].
#[cfg(feature = "utoipa")]
mod utoipa_impls
{
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::schema::OneOfBuilder;
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::JumpType;

	impl<'s> ToSchema<'s> for JumpType
	{
		fn schema() -> (&'s str, RefOr<Schema>)
		{
			(
				"JumpType",
				Schema::OneOf(
					OneOfBuilder::new()
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

	impl IntoParams for JumpType
	{
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
		{
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
