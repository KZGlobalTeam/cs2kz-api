//! Course difficulty ratings for CS2KZ.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// All the different course difficulties.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier
{
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

	/// For players with a lot of KZ experience who want to challenge
	/// themselves. Getting a top time on these requires mastering KZ.
	Extreme = 7,

	/// These are the hardest in the game, and only very good KZ players can
	/// complete these at all.
	Death = 8,

	/// Technically possible, but not feasible for humans. This tier is reserved
	/// for TAS runs, and any runs submitted by humans will be reviewed for
	/// cheats.
	Unfeasible = 9,

	/// Technically impossible. Even with perfect inputs.
	Impossible = 10,
}

impl Tier
{
	/// Returns a string representation of this [Tier], as accepted by the API.
	pub const fn as_str(&self) -> &'static str
	{
		match *self {
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

impl Display for Tier
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
	{
		f.write_str(match *self {
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

/// Error for parsing a string into a [`Tier`].
#[derive(Debug, Clone, Error)]
#[error("unrecognized tier `{0}`")]
pub struct UnknownTier(pub String);

impl FromStr for Tier
{
	type Err = UnknownTier;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		let s = s.to_lowercase();

		match s.as_str() {
			"very_easy" | "very easy" => Ok(Self::VeryEasy),
			"easy" => Ok(Self::Easy),
			"medium" => Ok(Self::Medium),
			"advanced" => Ok(Self::Advanced),
			"hard" => Ok(Self::Hard),
			"very_hard" | "very hard" => Ok(Self::VeryHard),
			"extreme" => Ok(Self::Extreme),
			"death" => Ok(Self::Death),
			"unfeasible" => Ok(Self::Unfeasible),
			"impossible" => Ok(Self::Impossible),
			_ => Err(UnknownTier(s)),
		}
	}
}

/// Error for parsing an integer into a [`Tier`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid tier `{0}`")]
pub struct InvalidTier(pub u8);

impl TryFrom<u8> for Tier
{
	type Error = InvalidTier;

	fn try_from(value: u8) -> Result<Self, Self::Error>
	{
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
			invalid => Err(InvalidTier(invalid)),
		}
	}
}

impl From<Tier> for u8
{
	#[expect(clippy::as_conversions, reason = "casts are required to turn enums into integers")]
	fn from(value: Tier) -> Self
	{
		value as u8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls
{
	use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

	use super::Tier;

	impl Serialize for Tier
	{
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			self.as_str().serialize(serializer)
		}
	}

	impl<'de> Deserialize<'de> for Tier
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

	use super::Tier;

	impl<DB> Type<DB> for Tier
	where
		DB: Database,
		u8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo
		{
			<u8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for Tier
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

	impl<'r, DB> Decode<'r, DB> for Tier
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

	use crate::Tier;

	impl<'s> ToSchema<'s> for Tier
	{
		fn schema() -> (&'s str, RefOr<Schema>)
		{
			(
				"Tier",
				Schema::OneOf(
					OneOfBuilder::new()
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

	impl IntoParams for Tier
	{
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
		{
			vec![ParameterBuilder::new()
				.name("tier")
				.parameter_in(parameter_in_provider().unwrap_or_default())
				.schema(Some(Self::schema().1))
				.build()]
		}
	}
}
