//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
use utoipa::{IntoParams, ToSchema};

use crate::Styles;

impl<'s> ToSchema<'s> for Styles
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		(
			"Styles",
			Schema::Object(
				ObjectBuilder::new()
					.title(Some("Name"))
					.schema_type(SchemaType::String)
					.example(Some("auto_bhop".into()))
					.enum_values(Some(Styles::ALL.iter_names()))
					.build(),
			)
			.into(),
		)
	}
}

impl IntoParams for Styles
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![ParameterBuilder::new()
			.parameter_in(parameter_in_provider().unwrap_or_default())
			.name("styles")
			.schema(Some(Self::schema().1))
			.build()]
	}
}
