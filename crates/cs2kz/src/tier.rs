//! Difficulty ratings for CS2KZ courses.

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// The 10 difficulty tiers for KZ courses.
///
/// Only the first 8 are considered "humanly possible". [Unfeasible] is technically possible,
/// but not realistically. [Impossible] is _actually_ impossible.
///
/// [Unfeasible]: type@Tier::Unfeasible
/// [Impossible]: type@Tier::Impossible
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
	/// The lowest tier.
	///
	/// Someone who has never played KZ before should be able to complete this.
	VeryEasy = 1,

	/// Requires some prior KZ knowledge, such as air strafing and bunnyhopping.
	Easy = 2,

	/// Players who have the KZ basics down should be able to complete this.
	Medium = 3,

	/// Players who have played KZ consistently for a while and are starting to
	/// learn more advanced techniques like ladders and surfs.
	Advanced = 4,

	/// Just like [Advanced], but harder.
	///
	/// [Advanced]: type@Tier::Advanced
	Hard = 5,

	/// Just like [Hard], but very.
	///
	/// [Hard]: type@Tier::Hard
	VeryHard = 6,

	/// For players with a lot of KZ experience who want to challenge themselves.
	/// Getting a top time on these requires mastering KZ.
	Extreme = 7,

	/// These are the hardest in the game, and only very good KZ players can
	/// complete these at all.
	Death = 8,

	/// Technically possible, but not feasible for humans. This tier is reserved for
	/// TAS runs, and any runs submitted by humans will be reviewed for cheats.
	Unfeasible = 9,

	/// Technically impossible. Even with perfect inputs.
	Impossible = 10,
}

impl Tier {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::VeryEasy => "very_easy",
			Self::Easy => "easy",
			Self::Medium => "medium",
			Self::Advanced => "advanced",
			Self::Hard => "hard",
			Self::VeryHard => "very_hard",
			Self::Extreme => "extreme",
			Self::Death => "death",
			Self::Unfeasible => "unfeasible",
			Self::Impossible => "impossible",
		}
	}
}

impl Display for Tier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::VeryEasy => "Very Easy",
			Self::Easy => "Easy",
			Self::Medium => "Medium",
			Self::Advanced => "Advanced",
			Self::Hard => "Hard",
			Self::VeryHard => "Very Hard",
			Self::Extreme => "Extreme",
			Self::Death => "Death",
			Self::Unfeasible => "Unfeasible",
			Self::Impossible => "Impossible",
		})
	}
}

impl From<Tier> for u8 {
	fn from(tier: Tier) -> Self {
		tier as u8
	}
}

impl TryFrom<u8> for Tier {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self> {
		match value {
			1 => Ok(Self::VeryEasy),
			2 => Ok(Self::Easy),
			3 => Ok(Self::Medium),
			4 => Ok(Self::Advanced),
			5 => Ok(Self::Hard),
			6 => Ok(Self::VeryHard),
			7 => Ok(Self::Extreme),
			8 => Ok(Self::Death),
			9 => Ok(Self::Unfeasible),
			10 => Ok(Self::Impossible),
			_ => Err(Error::InvalidTier),
		}
	}
}

impl FromStr for Tier {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<u8>() {
			return Self::try_from(value);
		}

		match value {
			"very easy" | "very_easy" => Ok(Self::VeryEasy),
			"easy" => Ok(Self::Easy),
			"medium" => Ok(Self::Medium),
			"advanced" => Ok(Self::Advanced),
			"hard" => Ok(Self::Hard),
			"very hard" | "very_hard" => Ok(Self::VeryHard),
			"extreme" => Ok(Self::Extreme),
			"death" => Ok(Self::Death),
			"unfeasible" => Ok(Self::Unfeasible),
			"impossible" => Ok(Self::Impossible),
			_ => Err(Error::InvalidTier),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::Tier;

		impl Tier {
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

		impl Serialize for Tier {
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

		use crate::Tier;

		impl Tier {
			/// Deserializes the value returned by [`Tier::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"very easy" | "very_easy" => Ok(Self::VeryEasy),
					"easy" => Ok(Self::Easy),
					"medium" => Ok(Self::Medium),
					"advanced" => Ok(Self::Advanced),
					"hard" => Ok(Self::Hard),
					"very hard" | "very_hard" => Ok(Self::VeryHard),
					"extreme" => Ok(Self::Extreme),
					"death" => Ok(Self::Death),
					"unfeasible" => Ok(Self::Unfeasible),
					"impossible" => Ok(Self::Impossible),
					value => Err(Error::invalid_value(U::Str(value), &"tier")),
				}
			}

			/// Deserializes from an integer.
			pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <u8>::deserialize(deserializer)? {
					1 => Ok(Self::VeryEasy),
					2 => Ok(Self::Easy),
					3 => Ok(Self::Medium),
					4 => Ok(Self::Advanced),
					5 => Ok(Self::Hard),
					6 => Ok(Self::VeryHard),
					7 => Ok(Self::Extreme),
					8 => Ok(Self::Death),
					9 => Ok(Self::Unfeasible),
					10 => Ok(Self::Impossible),
					value => Err(Error::invalid_value(
						U::Unsigned(value as u64),
						&"value between 1 and 10",
					)),
				}
			}
		}

		impl<'de> Deserialize<'de> for Tier {
			/// Best-effort attempt at deserializing a [`Tier`] of unknown format.
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

	use crate::Tier;

	impl<DB> Type<DB> for Tier
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for Tier
	where
		DB: Database,
		u8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			(*self as u8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for Tier
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

	use crate::Tier;

	impl<'s> ToSchema<'s> for Tier {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"Tier",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("hard".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("hard".into()))
								.enum_values(Some([
									"very_easy",
									"easy",
									"medium",
									"advanced",
									"hard",
									"very_hard",
									"extreme",
									"death",
									"unfeasible",
									"impossible",
								]))
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Numeric Value"))
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

	impl IntoParams for Tier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("tier")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
