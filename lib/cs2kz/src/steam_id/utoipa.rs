//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{KnownFormat, ObjectBuilder, OneOfBuilder, RefOr, SchemaFormat, SchemaType};
use utoipa::{IntoParams, PartialSchema, ToSchema};

use crate::SteamID;

impl PartialSchema for SteamID
{
	fn schema() -> RefOr<Schema>
	{
		Schema::OneOf(
			OneOfBuilder::new()
				.description(Some("a player's SteamID"))
				.example(Some("STEAM_1:1:161178172".into()))
				.item(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::String)
						.title(Some("SteamID"))
						.example(Some("STEAM_1:1:161178172".into()))
						.pattern(Some(r#"^STEAM_[0|1]:[0|1]:\d+$"#))
						.build(),
				))
				.item(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::Integer)
						.title(Some("SteamID64"))
						.example(Some("76561198282622073".into()))
						.format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt64)))
						.minimum(Some(super::MIN as f64))
						.maximum(Some(super::MAX as f64))
						.build(),
				))
				.item(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::Integer)
						.title(Some("SteamID32"))
						.example(Some("322356345".into()))
						.format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt32)))
						.minimum(Some((super::MIN - super::MAGIC_OFFSET) as f64))
						.maximum(Some((super::MAX - super::MAGIC_OFFSET) as f64))
						.build(),
				))
				.build(),
		)
		.into()
	}
}

impl<'s> ToSchema<'s> for SteamID
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		("SteamID", <Self as PartialSchema>::schema())
	}
}

impl IntoParams for SteamID
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![
			ParameterBuilder::new()
				.name("steam_id")
				.parameter_in(parameter_in_provider().unwrap_or_default())
				.description(Some("a player's SteamID"))
				.schema(Some(<Self as PartialSchema>::schema()))
				.example(Some("76561198282622073".into()))
				.build(),
		]
	}
}
