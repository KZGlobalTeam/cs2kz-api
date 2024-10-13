//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, SchemaType};
use utoipa::{IntoParams, PartialSchema, ToSchema};

use crate::Mode;

impl PartialSchema for Mode
{
	fn schema() -> RefOr<Schema>
	{
		Schema::OneOf(
			OneOfBuilder::new()
				.description(Some("a KZ Mode"))
				.example(Some("ckz".into()))
				.item(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::String)
						.title(Some("Name"))
						.example(Some("classic".into()))
						.enum_values(Some(["vanilla", "classic"]))
						.build(),
				))
				.item(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::Integer)
						.title(Some("ID"))
						.example(Some("1".into()))
						.enum_values(Some([1, 2]))
						.build(),
				))
				.build(),
		)
		.into()
	}
}

impl<'s> ToSchema<'s> for Mode
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		("Mode", <Self as PartialSchema>::schema())
	}
}

impl IntoParams for Mode
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![ParameterBuilder::new()
			.name("mode")
			.parameter_in(parameter_in_provider().unwrap_or_default())
			.description(Some("a KZ mode"))
			.schema(Some(<Self as PartialSchema>::schema()))
			.example(Some("classic".into()))
			.build()]
	}
}
