macro_rules! into_params {
	($type:ty as $name:literal : $description:literal) => {
		impl utoipa::IntoParams for $type {
			fn into_params(
				parameter_in_provider: impl Fn() -> Option<utoipa::openapi::path::ParameterIn>,
			) -> Vec<utoipa::openapi::path::Parameter> {
				vec![
					utoipa::openapi::path::ParameterBuilder::new()
						.name($name)
						.parameter_in(parameter_in_provider().unwrap_or_default())
						.description(Some($description))
						.schema(Some(<Self as utoipa::ToSchema>::schema().1))
						.build(),
				]
			}
		}
	};
}

pub(crate) use into_params;
