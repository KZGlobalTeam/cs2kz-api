use std::panic::Location;

use axum::response::{IntoResponse, Response};
use headers::Header;

use crate::problem_details::{ProblemDetails, ProblemType};
use crate::steam;

/// The standard error response returned by handlers.
#[derive(Debug)]
pub struct ErrorResponse(ErrorKind);

#[derive(Debug)]
enum ErrorKind {
    Unauthorized,
    NotFound,
    FailedToBufferBody,
    InternalServerError,
    BadGateway,

    #[debug("{:?}", _0.problem_type())]
    Detailed(ProblemDetails),
}

impl ErrorResponse {
    pub(crate) fn detailed(details: ProblemDetails) -> Self {
        Self(ErrorKind::Detailed(details))
    }

    pub(crate) fn unauthorized() -> Self {
        Self(ErrorKind::Unauthorized)
    }

    pub(crate) fn not_found() -> Self {
        Self(ErrorKind::NotFound)
    }

    pub(crate) fn failed_to_buffer_body() -> Self {
        Self(ErrorKind::FailedToBufferBody)
    }

    #[track_caller]
    pub(crate) fn internal_server_error<E>(error: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        error!(
            error = &error as &dyn std::error::Error,
            loc = %Location::caller(),
            "internal server error",
        );

        Self(ErrorKind::InternalServerError)
    }

    #[track_caller]
    pub(crate) fn bad_gateway<E>(error: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        warn!(
            error = &error as &dyn std::error::Error,
            loc = %Location::caller(),
            "failed to call external service",
        );

        Self(ErrorKind::BadGateway)
    }

    pub(crate) fn missing_header<H: Header>() -> Self {
        Self::detailed(problem_details(ProblemType::MissingHeader, |details| {
            details.add_extension("header", H::name().as_str());
        }))
    }

    pub(crate) fn invalid_header<H: Header>(mut modify: impl FnMut(&mut ProblemDetails)) -> Self {
        Self::detailed(problem_details(ProblemType::InvalidHeader, |details| {
            details.add_extension("header", H::name().as_str());
            modify(details);
        }))
    }

    pub(crate) fn invalid_path_params(modify: impl FnMut(&mut ProblemDetails)) -> Self {
        Self::detailed(problem_details(ProblemType::InvalidPathParameters, modify))
    }

    pub(crate) fn invalid_query_string(modify: impl FnMut(&mut ProblemDetails)) -> Self {
        Self::detailed(problem_details(ProblemType::InvalidQueryString, modify))
    }

    pub(crate) fn plugin_version_already_exists() -> Self {
        Self::detailed(problem_details(ProblemType::PluginVersionAlreadyExists, |_| {}))
    }

    pub(crate) fn outdated_plugin_version(latest: &semver::Version) -> Self {
        Self::detailed(problem_details(ProblemType::OutdatedPluginVersion, |details| {
            details.add_extension("latest_plugin_version", latest);
        }))
    }

    pub(crate) fn server_name_already_taken() -> Self {
        Self::detailed(problem_details(ProblemType::ServerNameAlreadyTaken, |_| {}))
    }

    pub(crate) fn server_host_and_port_already_taken() -> Self {
        Self::detailed(problem_details(ProblemType::ServerHostAndPortAlreadyTaken, |_| {}))
    }

    pub(crate) fn server_owner_does_not_exist() -> Self {
        Self::detailed(problem_details(ProblemType::ServerOwnerDoesNotExist, |_| {}))
    }

    pub(crate) fn server_owner_cannot_reactivate_server() -> Self {
        Self::detailed(problem_details(ProblemType::ServerOwnerCannotReactivateServer, |_| {}))
    }

    pub(crate) fn invalid_request_body(modify: impl FnMut(&mut ProblemDetails)) -> Self {
        Self::detailed(problem_details(ProblemType::InvalidRequestBody, modify))
    }

    pub(crate) fn map_must_have_mappers() -> Self {
        Self::detailed(problem_details(ProblemType::MapMustHaveMappers, |_| {}))
    }

    pub(crate) fn invalid_course_index(idx: usize) -> Self {
        Self::detailed(problem_details(ProblemType::InvalidCourseIndex, |details| {
            details.set_detail(format!("map has no course #{idx}"));
        }))
    }

    pub(crate) fn player_already_banned() -> Self {
        Self::detailed(problem_details(ProblemType::PlayerAlreadyBanned, |_| {}))
    }
}

fn problem_details(
    problem_type: ProblemType,
    mut modify: impl FnMut(&mut ProblemDetails),
) -> ProblemDetails {
    let mut problem_details = ProblemDetails::new(problem_type);
    modify(&mut problem_details);
    problem_details
}

impl From<steam::ApiError> for ErrorResponse {
    #[track_caller]
    fn from(error: steam::ApiError) -> Self {
        match error {
            steam::ApiError::Http(error) => ErrorResponse::internal_server_error(error),
            steam::ApiError::BufferResponseBody { error, response } => {
                debug!(%response.status);
                ErrorResponse::internal_server_error(error)
            },
            steam::ApiError::DeserializeResponse { error, response } => {
                debug!(response.status = %response.status(), response.body = ?response.body());
                ErrorResponse::internal_server_error(error)
            },
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        match self.0 {
            ErrorKind::Unauthorized => http::StatusCode::UNAUTHORIZED.into_response(),
            ErrorKind::NotFound => http::StatusCode::NOT_FOUND.into_response(),
            ErrorKind::FailedToBufferBody => http::StatusCode::PAYLOAD_TOO_LARGE.into_response(),
            ErrorKind::InternalServerError => {
                http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            },
            ErrorKind::BadGateway => http::StatusCode::BAD_GATEWAY.into_response(),
            ErrorKind::Detailed(details) => details.into(),
        }
    }
}
