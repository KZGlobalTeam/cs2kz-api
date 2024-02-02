use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as AxumResponse};
use utoipa::openapi::response::Response as OpenApiResponse;
use utoipa::openapi::RefOr;
use utoipa::{IntoResponses, ToSchema};

#[derive(IntoResponses)]
#[response(status = OK)]
pub struct Ok<T: ToSchema<'static>>(#[to_schema] T);

/// Wrapper struct for turning any `T` into a [Response] with status code 201.
///
/// [Response]: axum::response::Response
pub struct Created<T>(pub T);

impl<T> IntoResponses for Created<T>
where
	T: ToSchema<'static>,
{
	fn responses() -> BTreeMap<String, RefOr<OpenApiResponse>> {
		#[derive(IntoResponses)]
		#[response(status = CREATED)]
		struct Helper<T: ToSchema<'static>>(#[to_schema] T);

		Helper::<T>::responses()
	}
}

impl<T> IntoResponse for Created<T>
where
	T: IntoResponse,
{
	fn into_response(self) -> AxumResponse {
		(StatusCode::CREATED, self.0).into_response()
	}
}

#[derive(IntoResponses)]
#[response(status = NO_CONTENT)]
pub struct NoContent;

impl IntoResponse for NoContent {
	fn into_response(self) -> AxumResponse {
		StatusCode::NO_CONTENT.into_response()
	}
}

#[derive(IntoResponses)]
#[response(status = SEE_OTHER)]
pub struct SeeOther;

#[derive(IntoResponses)]
#[response(status = BAD_REQUEST)]
pub struct BadRequest;

#[derive(IntoResponses)]
#[response(status = UNAUTHORIZED)]
pub struct Unauthorized;

#[derive(IntoResponses)]
#[response(status = CONFLICT)]
pub struct Conflict;

#[derive(IntoResponses)]
#[response(status = UNPROCESSABLE_ENTITY)]
pub struct UnprocessableEntity;

#[derive(IntoResponses)]
#[response(status = INTERNAL_SERVER_ERROR, description = "Something unexpected happened. This is a bug; please report it.")]
pub struct InternalServerError;

#[derive(IntoResponses)]
#[response(status = BAD_GATEWAY, description = "Communication with an external service failed (e.g. Steam).")]
pub struct BadGateway;
