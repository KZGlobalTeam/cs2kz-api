//! Different ways of identifying map courses.

crate::identifier::identifier! {
	/// Different ways of identifying a map course.
	enum CourseIdentifier {
		/// An ID.
		ID(u16),

		/// A name.
		Name(String),
	}

	ParseError: ParseCourseIdentifierError
}

/// Method and Trait implementations when depending on [`utoipa`].
#[cfg(feature = "utoipa")]
mod utoipa_impls {
	use utoipa::openapi::path::{Parameter, ParameterBuilder, ParameterIn};
	use utoipa::openapi::{ObjectBuilder, OneOfBuilder, RefOr, Schema, SchemaType};
	use utoipa::{IntoParams, ToSchema};

	use crate::CourseIdentifier;

	impl<'s> ToSchema<'s> for CourseIdentifier {
		fn schema() -> (&'s str, RefOr<Schema>) {
			(
				"CourseIdentifier",
				Schema::OneOf(
					OneOfBuilder::new()
						.description(Some("A course ID or name"))
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

	impl IntoParams for CourseIdentifier {
		fn into_params(parameter_in_provider: impl Fn() -> Option<ParameterIn>) -> Vec<Parameter> {
			vec![
				ParameterBuilder::new()
					.name("course")
					.description(Some("A course ID or name"))
					.parameter_in(parameter_in_provider().unwrap_or_default())
					.schema(Some(Self::schema().1))
					.build(),
			]
		}
	}
}
