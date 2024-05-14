//! Ranked Status for course filters.
//!
//! Every global map is made up of 1 or more courses. Each course has 4 filters, and each filter
//! has its own [ranked status]. This determines whether players gain points from those filters.
//!
//! [ranked status]: RankedStatus

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

/// The ranked status of a course filter.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum RankedStatus {
	/// The filter will never be [ranked], as per the mapper's request.
	///
	/// [ranked]: Self::Ranked
	Never = -1,

	/// The filter is currently not ranked, but not because it was explicitly
	/// requested, just because it didn't meet requirements.
	Unranked = 0,

	/// The filter is ranked.
	Ranked = 1,
}

impl RankedStatus {
	/// Checks whether `self` is [Ranked].
	///
	/// [Ranked]: Self::Ranked
	pub const fn is_ranked(&self) -> bool {
		matches!(*self, Self::Ranked)
	}

	/// Returns a string representation of this [RankedStatus], as accepted by the API.
	pub const fn as_str(&self) -> &'static str {
		match *self {
			Self::Never => "never",
			Self::Unranked => "unranked",
			Self::Ranked => "ranked",
		}
	}
}

impl Display for RankedStatus {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// Error for parsing a string into a [`RankedStatus`].
#[derive(Debug, Clone, Error)]
#[error("unknown ranked status `{0}`")]
pub struct UnknownRankedStatus(pub String);

impl FromStr for RankedStatus {
	type Err = UnknownRankedStatus;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();

		match s.as_str() {
			"never" => Ok(Self::Never),
			"unranked" => Ok(Self::Unranked),
			"ranked" => Ok(Self::Ranked),
			_ => Err(UnknownRankedStatus(s)),
		}
	}
}

/// Error for converting an integer to a [`RankedStatus`].
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid ranked status `{0}`")]
pub struct InvalidRankedStatus(pub i8);

impl TryFrom<i8> for RankedStatus {
	type Error = InvalidRankedStatus;

	fn try_from(value: i8) -> Result<Self, Self::Error> {
		match value {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			invalid => Err(InvalidRankedStatus(invalid)),
		}
	}
}

impl From<RankedStatus> for i8 {
	#[allow(clippy::as_conversions)]
	fn from(value: RankedStatus) -> Self {
		value as i8
	}
}

/// Method and Trait implementations when depending on [`serde`].
#[cfg(feature = "serde")]
mod serde_impls {
	use serde::{de, Deserialize, Deserializer};

	use super::RankedStatus;

	impl<'de> Deserialize<'de> for RankedStatus {
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

	use super::RankedStatus;

	impl<DB> Type<DB> for RankedStatus
	where
		DB: Database,
		i8: Type<DB>,
	{
		fn type_info() -> <DB as Database>::TypeInfo {
			<i8 as Type<DB>>::type_info()
		}
	}

	impl<'q, DB> Encode<'q, DB> for RankedStatus
	where
		DB: Database,
		i8: Encode<'q, DB>,
	{
		fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
			<i8 as Encode<'q, DB>>::encode_by_ref(&i8::from(*self), buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for RankedStatus
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

	use crate::RankedStatus;

	impl<'s> ToSchema<'s> for RankedStatus {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"RankedStatus",
				Schema::AnyOf(
					AnyOfBuilder::new()
						.nullable(false)
						.example(Some("ranked".into()))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.example(Some("ranked".into()))
								.enum_values(Some(["never", "unranked", "ranked"]))
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

	impl IntoParams for RankedStatus {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("ranked_status")
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
