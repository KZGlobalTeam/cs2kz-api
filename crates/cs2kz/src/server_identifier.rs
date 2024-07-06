//! Different ways of identifying servers.

crate::identifier::identifier! {
	/// Different ways of identifying a server.
	enum ServerIdentifier {
		/// An ID.
		ID(u16),

		/// A name.
		Name(String),
	}

	ParseError: ParseServerIdentifierError
}

/// Method and Trait implementations when depending on [`utoipa`].
#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::ServerIdentifier;

	impl<'s> ToSchema<'s> for ServerIdentifier {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"ServerIdentifier",
				Schema::OneOf(
					OneOfBuilder::new()
						.description(Some("A server ID or name"))
						.nullable(false)
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("ID"))
								.schema_type(SchemaType::Integer)
								.build(),
						))
						.item(Schema::Object(
							ObjectBuilder::new()
								.title(Some("Name"))
								.schema_type(SchemaType::String)
								.build(),
						))
						.build(),
				)
				.into(),
			)
		}
	}

	impl IntoParams for ServerIdentifier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("server")
					.description(Some("A server ID or name"))
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
