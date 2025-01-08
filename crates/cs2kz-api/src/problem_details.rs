use std::fmt;

pub type ProblemDetails = problem_details::ProblemDetails<ProblemType>;

#[derive(Debug, PartialEq, Eq)]
pub enum ProblemType {
    MissingHeader,
    InvalidHeader,
    InvalidPathParameters,
    InvalidQueryString,
    PluginVersionAlreadyExists,
    OutdatedPluginVersion,
    ServerNameAlreadyTaken,
    ServerHostAndPortAlreadyTaken,
    ServerOwnerDoesNotExist,
    ServerOwnerCannotReactivateServer,
    MapMustHaveMappers,
    InvalidCourseIndex,
    PlayerAlreadyBanned,
    InvalidRequestBody,
}

macro uri($fragment:literal) {
    http::Uri::from_static(concat!("https://docs.cs2kz.org/api/problems#", $fragment))
}

impl problem_details::ProblemType for ProblemType {
    fn uri(&self) -> http::Uri {
        match self {
            Self::MissingHeader => uri!("missing-header"),
            Self::InvalidHeader => uri!("invalid-header"),
            Self::InvalidPathParameters => uri!("invalid-path-parameters"),
            Self::InvalidQueryString => uri!("invalid-query-string"),
            Self::PluginVersionAlreadyExists => uri!("plugin-version-already-exists"),
            Self::OutdatedPluginVersion => uri!("outdated-plugin-version"),
            Self::ServerNameAlreadyTaken => uri!("server-name-already-taken"),
            Self::ServerHostAndPortAlreadyTaken => uri!("server-host-and-port-already-taken"),
            Self::ServerOwnerDoesNotExist => uri!("server-owner-does-not-exist"),
            Self::ServerOwnerCannotReactivateServer => {
                uri!("server-owner-cannot-reactivate-server")
            },
            Self::MapMustHaveMappers => uri!("map-must-have-mappers"),
            Self::InvalidCourseIndex => uri!("invalid-course-index"),
            Self::PlayerAlreadyBanned => uri!("player-already-banned"),
            Self::InvalidRequestBody => uri!("invalid-request-body"),
        }
    }

    fn status(&self) -> http::StatusCode {
        match self {
            Self::MissingHeader
            | Self::InvalidHeader
            | Self::InvalidPathParameters
            | Self::InvalidQueryString => http::StatusCode::BAD_REQUEST,
            Self::ServerOwnerCannotReactivateServer => http::StatusCode::UNAUTHORIZED,
            Self::PluginVersionAlreadyExists
            | Self::OutdatedPluginVersion
            | Self::ServerNameAlreadyTaken
            | Self::ServerHostAndPortAlreadyTaken
            | Self::ServerOwnerDoesNotExist
            | Self::MapMustHaveMappers
            | Self::InvalidCourseIndex
            | Self::PlayerAlreadyBanned => http::StatusCode::CONFLICT,
            Self::InvalidRequestBody => http::StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn title(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => write!(fmt, "missing required header"),
            Self::InvalidHeader => write!(fmt, "failed to parse header value"),
            Self::InvalidPathParameters => write!(fmt, "failed to parse path parameter(s)"),
            Self::InvalidQueryString => write!(fmt, "failed to parse query string"),
            Self::PluginVersionAlreadyExists => {
                write!(fmt, "this plugin version has already been published")
            },
            Self::OutdatedPluginVersion => {
                write!(fmt, "the latest plugin version is newer than the version submitted")
            },
            Self::ServerNameAlreadyTaken => write!(fmt, "server name is already taken"),
            Self::ServerHostAndPortAlreadyTaken => {
                write!(fmt, "host+port combination is already in use")
            },
            Self::ServerOwnerDoesNotExist => write!(fmt, "server owner does not exist"),
            Self::ServerOwnerCannotReactivateServer => write!(
                fmt,
                "you are not allowed to generate a new access key if you do not currently have one",
            ),
            Self::MapMustHaveMappers => {
                write!(fmt, "maps and courses must always have at least one mapper")
            },
            Self::InvalidCourseIndex => write!(fmt, "invalid index for course in map update"),
            Self::PlayerAlreadyBanned => write!(fmt, "player is already banned"),
            Self::InvalidRequestBody => write!(fmt, "failed to parse request body"),
        }
    }
}
