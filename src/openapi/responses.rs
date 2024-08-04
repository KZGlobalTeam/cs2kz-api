//! [`utoipa::IntoResponses`] template implementations for other types.

/// Generates an implementation for [`utoipa::IntoResponses`] using a 204 status
/// code.
///
/// # Example
///
/// ```ignore
/// struct MyResponse { /* … */ }
///
/// crate::openapi::responses::no_content!(MyResponse);
/// ```
macro_rules! no_content {
	($ty:ty) => {
		impl ::utoipa::IntoResponses for $ty
		{
			fn responses() -> ::std::collections::BTreeMap<
				::std::string::String,
				::utoipa::openapi::RefOr<::utoipa::openapi::response::Response>,
			>
			{
				::utoipa::openapi::response::ResponsesBuilder::new()
					.response("204", ::utoipa::openapi::response::ResponseBuilder::new())
					.build()
					.into()
			}
		}
	};
}

pub(crate) use no_content;
