use utoipa::{IntoResponses, ToSchema};

#[derive(IntoResponses)]
#[response(
	status = StatusCode::OK,
	description = "The request was successful.",
)]
pub struct Ok<T>(#[to_schema] T)
where
	T: for<'a> ToSchema<'a>;

#[derive(IntoResponses)]
#[response(
	status = StatusCode::CREATED,
	description = "The request was successful.",
)]
pub struct Created<T>(#[to_schema] T)
where
	T: for<'a> ToSchema<'a>;

#[derive(IntoResponses)]
#[response(
	status = StatusCode::NO_CONTENT,
	description = "There is no data available for the given query.",
)]
pub struct NoContent;

#[derive(IntoResponses)]
#[response(
	status = StatusCode::UNAUTHORIZED,
	description = "You do not have access to this resource.",
)]
pub struct Unauthorized;

#[derive(IntoResponses)]
#[response(
	status = StatusCode::BAD_REQUEST,
	description = "Something about the request was incorrect.",
)]
pub struct BadRequest;

#[derive(IntoResponses)]
#[response(
	status = StatusCode::INTERNAL_SERVER_ERROR,
	description = "A bug in the API.",
)]
pub struct InternalServerError;
