//! Trait implementations for the [`utoipa`] crate.

use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::{Array, OneOfBuilder};
use utoipa::openapi::{ObjectBuilder, RefOr, Schema, SchemaType};
use utoipa::{IntoParams, ToSchema as _ToSchema};

use super::Permissions;

impl<'s> _ToSchema<'s> for Permissions
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		(
			"Permissions",
			Schema::Array(Array::new(Schema::OneOf(
				OneOfBuilder::new()
					.nullable(false)
					.example(Some("maps".into()))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("Name"))
							.schema_type(SchemaType::String)
							.example(Some("maps".into()))
							.enum_values(Some(Permissions::ALL.iter_names()))
							.build(),
					))
					.item(Schema::Object(
						ObjectBuilder::new()
							.title(Some("ID"))
							.schema_type(SchemaType::Integer)
							.example(Some((1 << 16).into()))
							.enum_values(Some(Permissions::ALL.iter_bits()))
							.build(),
					))
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
		vec![
			ParameterBuilder::new()
				.parameter_in(parameter_in_provider().unwrap_or_default())
				.name("permissions")
				.schema(Some(Self::schema().1))
				.build(),
		]
	}
}
