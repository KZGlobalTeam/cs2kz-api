//! This module holds structs that are passed to [`utoipa`] macros for building OpenAPI docs.
//! They are not used at runtime.

// These should all be self-explanatory.
#![allow(missing_debug_implementations, missing_docs)]

use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::OpenApi;
use utoipa::{IntoResponses, Modify, ToSchema};

#[derive(IntoResponses)]
#[response(status = StatusCode::OK, description = "The request was successful.")]
pub struct Ok<T: ToSchema<'static> = ()>(#[to_schema] T);

#[derive(IntoResponses)]
#[response(status = StatusCode::CREATED, description = "A resource has been created.")]
pub struct Created<T: ToSchema<'static> = ()>(#[to_schema] T);

#[derive(IntoResponses)]
#[response(status = StatusCode::NO_CONTENT, description = "There was no data available for the given query.")]
pub struct NoContent;

#[derive(IntoResponses)]
#[response(status = StatusCode::SEE_OTHER, description = "This is a redirect.")]
pub struct Redirect;

#[derive(IntoResponses)]
#[response(status = StatusCode::BAD_REQUEST, description = "Required request data was missing / invalid.")]
pub struct BadRequest;

#[derive(IntoResponses)]
#[response(status = StatusCode::UNAUTHORIZED, description = "You do not have access to this resource.")]
pub struct Unauthorized;

#[derive(IntoResponses)]
#[response(status = StatusCode::CONFLICT, description = "The request conflicts with the server's state. This can happen, for example, when trying to create a resource that already exists.")]
pub struct Conflict;

#[derive(IntoResponses)]
#[response(status = StatusCode::INTERNAL_SERVER_ERROR, description = "A bug in the API.")]
pub struct InternalServerError;

pub struct Security;

impl Modify for Security {
	fn modify(&self, openapi: &mut OpenApi) {
		openapi
			.components
			.as_mut()
			.expect("The API has components defined.")
			.add_security_scheme(
				"GameServer JWT",
				SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
			)
	}
}
