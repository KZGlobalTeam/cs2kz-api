//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::Array;
use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
use utoipa::{IntoParams, ToSchema as _ToSchema};

use super::Permissions;

impl<'s> _ToSchema<'s> for Permissions
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		(
			"Permissions",
			Schema::Array(Array::new(Schema::Object(
				ObjectBuilder::new()
					.title(Some("Name"))
					.schema_type(SchemaType::String)
					.example(Some("maps".into()))
					.enum_values(Some(Permissions::ALL.iter_names()))
					.build(),
			)))
			.into(),
		)
	}
}

impl IntoParams for Permissions
{
	fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter>
	{
		vec![ParameterBuilder::new()
			.parameter_in(parameter_in_provider().unwrap_or_default())
			.name("permissions")
			.schema(Some(Self::schema().1))
			.build()]
	}
}
