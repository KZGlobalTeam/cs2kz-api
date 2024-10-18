//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::{IntoParams, PartialSchema, ToSchema};

use crate::Mode;

impl PartialSchema for Mode
{
	fn schema() -> RefOr<Schema>
	{
		Schema::Object(
			ObjectBuilder::new()
				.schema_type(SchemaType::String)
				.title(Some("Name"))
				.example(Some("classic".into()))
				.enum_values(Some(["vanilla", "classic"]))
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
