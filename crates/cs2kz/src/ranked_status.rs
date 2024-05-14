//! Ranked Status for KZ course filters.
//!
//! Every global map is made up of 1 or more courses. Each course has 4 filters, and each filter
//! has its own [ranked status]. This determines whether players gain points from those filters.
//!
//! [ranked status]: RankedStatus

use std::fmt::{self, Display};
use std::str::FromStr;

use crate::{Error, Result};

/// The ranked status of a course filter.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RankedStatus {
	/// The filter will never be ranked; either because the mapper requested it, or
	/// because the map approval team decided so.
	Never = -1,

	/// The filter is currently not ranked; probably because it didn't meet
	/// requirements.
	Unranked = 0,

	/// The filter is ranked.
	Ranked = 1,
}

impl RankedStatus {
	/// A string format compatible with the API.
	#[inline]
	pub const fn api(&self) -> &'static str {
		match self {
			Self::Never => "never",
			Self::Unranked => "unranked",
			Self::Ranked => "ranked",
		}
	}
}

impl Display for RankedStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl From<RankedStatus> for i8 {
	fn from(ranked_status: RankedStatus) -> Self {
		ranked_status as i8
	}
}

impl TryFrom<i8> for RankedStatus {
	type Error = Error;

	fn try_from(value: i8) -> Result<Self> {
		match value {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			_ => Err(Error::InvalidRankedStatus),
		}
	}
}

impl FromStr for RankedStatus {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		if let Ok(value) = value.parse::<i8>() {
			return Self::try_from(value);
		}

		match value {
			"never" => Ok(Self::Never),
			"unranked" => Ok(Self::Unranked),
			"ranked" => Ok(Self::Ranked),
			_ => Err(Error::InvalidRankedStatus),
		}
	}
}

#[cfg(feature = "serde")]
mod serde_impls {
	mod ser {
		use serde::{Serialize, Serializer};

		use crate::RankedStatus;

		impl RankedStatus {
			/// Serialize in a API compatible format.
			pub fn serialize_api<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				self.api().serialize(serializer)
			}

			/// Serialize this ranked status as an integer value.
			pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				(*self as i8).serialize(serializer)
			}
		}

		impl Serialize for RankedStatus {
			/// Uses the [`RankedStatus::serialize_api()`] method.
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

		use crate::RankedStatus;

		impl RankedStatus {
			/// Deserializes the value returned by [`RankedStatus::api()`].
			pub fn deserialize_api<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <&'de str>::deserialize(deserializer)? {
					"never" => Ok(Self::Never),
					"unranked" => Ok(Self::Unranked),
					"ranked" => Ok(Self::Ranked),
					value => Err(Error::invalid_value(
						U::Str(value),
						&"`never` | `unranked` | `ranked`",
					)),
				}
			}

			/// Deserializes a ranked status integer value.
			pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				match <i8>::deserialize(deserializer)? {
					-1 => Ok(Self::Never),
					0 => Ok(Self::Unranked),
					1 => Ok(Self::Ranked),
					value => Err(Error::invalid_value(
						U::Signed(value as i64),
						&"-1, 0, or 1",
					)),
				}
			}
		}

		impl<'de> Deserialize<'de> for RankedStatus {
			/// Best-effort attempt at deserializing a [`RankedStatus`] of unknown
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

	use crate::RankedStatus;

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
			(*self as i8).encode_by_ref(buf)
		}
	}

	impl<'r, DB> Decode<'r, DB> for RankedStatus
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
