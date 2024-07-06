//! The gamemodes available in CS2KZ.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// All official gamemodes included in the CS2KZ plugin.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Mode {
	/// The VNL gamemode.
	Vanilla = 1,

	/// The CKZ gamemode.
	Classic = 2,
}

impl Mode {
	/// Checks whether `self` is of the [Vanilla] variant.
	///
	/// [Vanilla]: Self::Vanilla
	pub const fn is_vanilla(&self) -> bool {
		matches!(*self, Self::Vanilla)
	}

	/// Checks whether `self` is of the [Classic] variant.
	///
	/// [Classic]: Self::Classic
	pub const fn is_classic(&self) -> bool {
		matches!(*self, Self::Classic)
	}

	/// Returns a string representation of this [Mode], as accepted by the API.
	pub const fn as_str(&self) -> &'static str {
		match *self {
			Self::Vanilla => "vanilla",
			Self::Classic => "classic",
		}
	}

	/// Returns a short string representation of this [Mode], as displayed in-game.
	pub const fn as_str_short(&self) -> &'static str {
		match *self {
			Self::Vanilla => "VNL",
			Self::Classic => "CKZ",
		}
	}
}

impl Display for Mode {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(match *self {
			Self::Vanilla => "Vanilla",
			Self::Classic => "Classic",
		})
	}
}

/// Error for parsing a string into a [`Mode`].
#[derive(Debug, Clone, Error)]
#[error("unrecognized mode `{0}`")]
pub struct UnknownMode(pub String);

impl FromStr for Mode {
	type Err = UnknownMode;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();

		match s.as_str() {
			"vnl" | "vanilla" => Ok(Self::Vanilla),
			"ckz" | "classic" => Ok(Self::Classic),
			_ => Err(UnknownMode(s)),
		}
	}
}

/// Error for converting a mode ID to a [`Mode`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid mode ID `{0}`")]
pub struct InvalidModeID(pub u8);

impl TryFrom<u8> for Mode {
	type Error = InvalidModeID;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(Self::Vanilla),
			2 => Ok(Self::Classic),
			invalid => Err(InvalidModeID(invalid)),
		}
	}
}

impl From<Mode> for u8 {
	#[allow(clippy::as_conversions)]
	fn from(value: Mode) -> Self {
		value as u8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{de, Deserialize, Deserializer};

	use super::Mode;

	impl<'de> Deserialize<'de> for Mode {
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

	use super::Mode;

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
			<u8 as Encode<'q, DB>>::encode_by_ref(&u8::from(*self), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for Mode
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
	use utoipa::openapi::schema::OneOfBuilder;
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::Mode;

	impl<'s> ToSchema<'s> for Mode {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"Mode",
				Schema::OneOf(
					OneOfBuilder::new()
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
