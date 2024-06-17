//! Runtime errors.
//!
//! This module exposes the [`Error`] type that is used across the code base for bubbling up
//! errors. Any foreign errors that can occur at runtime can be turned into an [`Error`]. Specific
//! error cases have dedicated constructors, see all the public methods on [`Error`].
//!
//! [`Error`] implements [`IntoResponse`], which means it can be returned from HTTP handlers,
//! middleware, etc.
//!
//! This module also exposes a [`Result`] type alias, which sets [`Error`] as the default `E` type
//! parameter.
//!
//! [`Error`]: struct@Error

use std::fmt::{self, Formatter};
use std::io;
use std::panic::Location;

use axum::extract::rejection::PathRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::typed_header::TypedHeaderRejection;
use derive_more::Display;
use itertools::Itertools;
use serde_json::json;
use thiserror::Error;

use crate::authorization::Permissions;
use crate::bans::{BanID, UnbanID};
use crate::make_id::ConvertIDError;
use crate::maps::{CourseID, FilterID, MapID};

/// Type alias for a [`Result<T, E>`] with its `E` parameter set to [`Error`].
///
/// [`Result`]: std::result::Result
/// [`Error`]: struct@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The API's core error type.
///
/// Any errors that ever reach the outside should be this type.
/// It carries information about the kind of error that occurred, where it occurred, and any extra
/// information like error sources or debug messages.
///
/// This type implements [`IntoResponse`], which means it can be returned from HTTP handlers,
/// middleware, etc.
#[derive(Debug, Error)]
pub struct Error {
	/// The kind of error that occurred.
	///
	/// This is used for determining the HTTP status code and error message for the response
	/// body, when an error is returned from a request.
	kind: ErrorKind,

	/// The source code location of where the error occurred.
	///
	/// This is used for debugging / troubleshooting, and is included in logs.
	location: Location<'static>,

	/// Extra information about the error, like source errors or debug messages.
	attachments: Vec<Attachment>,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let Self {
			kind,
			location,
			attachments,
		} = self;

		write!(f, "[{location}] {kind}")?;

		if !attachments.is_empty() {
			write!(f, ":")?;

			for attachment in attachments.iter().rev() {
				write!(f, "\n  - {attachment}")?;
			}
		}

		Ok(())
	}
}

#[allow(clippy::missing_docs_in_private_items)]
const UNAUTHORIZED_MSG: &str = "you are not permitted to perform this action";

/// The different kinds of errors that can occur at runtime.
///
/// Every individual error case should be covered by this enum, with its own error message and any
/// extra information that is necessary to keep around.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Error)]
enum ErrorKind {
	#[error("no content")]
	NoContent,

	#[error("could not find {what}")]
	NotFound { what: String },

	#[error("invalid {what}")]
	InvalidInput { what: String },

	#[error("{UNAUTHORIZED_MSG}")]
	Unauthorized,

	#[error("this access key is expired")]
	ExpiredAccessKey,

	#[error("you are not logged in")]
	MissingSessionID,

	#[error("{UNAUTHORIZED_MSG}")]
	InsufficientPermissions { required_permissions: Permissions },

	#[error("{UNAUTHORIZED_MSG}")]
	MustBeServerOwner,

	#[error("{what} already exists")]
	AlreadyExists { what: &'static str },

	#[error("map/course cannot have 0 mappers")]
	MustHaveMappers,

	#[error("mismatching map/course ids; course `{course_id}` does not belong to map `{map_id}`")]
	MismatchingMapCourse { course_id: CourseID, map_id: MapID },

	#[error(
		"mismatching course/filter ids; filter `{filter_id}` does not belong to course `{course_id}`"
	)]
	MismatchingCourseFilter {
		filter_id: FilterID,
		course_id: CourseID,
	},

	#[error("ban `{ban_id}` was already reverted by unban `{unban_id}`")]
	BanAlreadyReverted { ban_id: BanID, unban_id: UnbanID },

	#[error("submitted plugin version {submitted} is outdated (latest is {latest})")]
	OutdatedPluginVersion {
		submitted: semver::Version,
		latest: semver::Version,
	},

	#[error("logic assertion failed: {0}")]
	Logic(String),

	#[cfg_attr(test, error("database error: {0}"))]
	#[cfg_attr(not(test), error("database error"))]
	Database(#[from] sqlx::Error),

	#[error("internal server error")]
	Jwt(jwt::errors::Error),

	#[error("internal server error")]
	Reqwest(#[from] reqwest::Error),

	#[error("missing workshop asset directory")]
	#[cfg(not(feature = "production"))]
	MissingWorkshopAssetDirectory,

	#[error("missing `DepotDownloader` binary")]
	#[cfg(not(feature = "production"))]
	MissingDepotDownloader,

	#[error("failed to run `DepotDownloader`")]
	DepotDownloader(io::Error),

	#[error("failed to download workshop map")]
	OpenMapFile(io::Error),

	#[error("failed to compute checksum for map")]
	Checksum(io::Error),

	#[error("external api call failed: {0}")]
	ExternalApiCall(reqwest::Error),

	#[error(transparent)]
	Header(#[from] TypedHeaderRejection),

	#[error(transparent)]
	Path(#[from] PathRejection),
}

#[allow(clippy::missing_docs_in_private_items)]
type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Generic error attachments.
#[derive(Debug, Display)]
#[display("'{context}' at {location}")]
struct Attachment {
	/// The attachment context.
	///
	/// This could be a more concrete error type, e.g. from a third party crate, or simply an
	/// error message.
	context: BoxedError,

	/// The source code location of where this attachment was created.
	location: Location<'static>,
}

impl Attachment {
	/// Creates a new [`Attachment`].
	#[track_caller]
	fn new<C>(context: C) -> Self
	where
		C: Into<BoxedError>,
	{
		Self {
			context: context.into(),
			location: *Location::caller(),
		}
	}
}

impl Error {
	/// Creates a new [`Error`] of the given [`ErrorKind`].
	///
	/// [`Error`]: struct@Error
	#[track_caller]
	fn new<E>(kind: E) -> Self
	where
		E: Into<ErrorKind>,
	{
		Self {
			kind: kind.into(),
			location: *Location::caller(),
			attachments: Vec::new(),
		}
	}

	/// Attach additional context to an error.
	///
	/// This can be another, more concrete, error type, or simply an error message.
	/// If `ctx` is also an [`Error`], it will have its attachments transferred to `self`.
	///
	/// [`Error`]: struct@Error
	#[track_caller]
	pub(crate) fn context<E>(mut self, ctx: E) -> Self
	where
		E: Into<BoxedError>,
	{
		match Into::<BoxedError>::into(ctx).downcast::<Self>() {
			Ok(mut err) => {
				self.attachments.append(&mut err.attachments);
				self.attachments.push(Attachment::new(err.kind));
			}
			Err(other) => {
				self.attachments.push(Attachment::new(other));
			}
		}

		self
	}

	/// A generic `204 No Content` error.
	///
	/// This should be returned from `PUT` / `PATCH` / `DELETE` handlers, as well as `GET`
	/// handlers that would otherwise return an empty response body.
	#[track_caller]
	pub(crate) fn no_content() -> Self {
		Self::new(ErrorKind::NoContent)
	}

	/// An error signaling that a resource could not be found.
	///
	/// Produces a `404 Not Found` status.
	#[track_caller]
	pub(crate) fn not_found<T>(what: T) -> Self
	where
		T: Display,
	{
		Self::new(ErrorKind::NotFound {
			what: what.to_string(),
		})
	}

	/// An error signaling invalid user input.
	///
	/// Produces a `400 Bad Request` status.
	#[track_caller]
	pub(crate) fn invalid<T>(what: T) -> Self
	where
		T: Display,
	{
		Self::new(ErrorKind::InvalidInput {
			what: what.to_string(),
		})
	}

	/// A generic `401 Unauthorized` error.
	///
	/// If you can, you should [attach additional context][context] to such an error to make
	/// debugging the cause of the error easier later.
	///
	/// [context]: Error::context()
	#[track_caller]
	pub(crate) fn unauthorized() -> Self {
		Self::new(ErrorKind::Unauthorized)
	}

	/// An error signaling an expired authentication key.
	///
	/// Produces a `401 Unauthorized` status.
	#[track_caller]
	pub(crate) fn expired_key() -> Self {
		Self::new(ErrorKind::ExpiredAccessKey)
	}

	/// An error signaling a missing session ID.
	///
	/// For more information about session authentication, see
	/// [`crate::authentication::session`].
	///
	/// Produces a `401 Unauthorized` status.
	#[track_caller]
	pub(crate) fn missing_session_id() -> Self {
		Self::new(ErrorKind::MissingSessionID)
	}

	/// An error signaling an authorization failure caused by insufficient permissions.
	///
	/// For more information about permissions, see [`crate::authorization::Permissions`] and
	/// [`crate::authorization::HasPermissions`].
	///
	/// Produces a `401 Unauthorized` status.
	#[track_caller]
	pub(crate) fn insufficient_permissions(required_permissions: Permissions) -> Self {
		Self::new(ErrorKind::InsufficientPermissions {
			required_permissions,
		})
	}

	/// An error signaling an authorization failure caused by the requesting user not
	/// being a server owner.
	///
	/// For more information, see [`crate::authorization::IsServerAdminOrOwner`].
	///
	/// Produces a `401 Unauthorized` status.
	#[track_caller]
	pub(crate) fn must_be_server_owner() -> Self {
		Self::new(ErrorKind::MustBeServerOwner)
	}

	/// An error signaling that a resource already exists.
	///
	/// Produces a `409 Conflict` status.
	#[track_caller]
	pub(crate) fn already_exists(what: &'static str) -> Self {
		Self::new(ErrorKind::AlreadyExists { what })
	}

	/// An error that can occur when creating or updating [maps].
	///
	/// Every map must always have at least 1 mapper. When a new map is submitted, or a map is
	/// being updated, it must be ensured that there is at least 1 mapper.
	///
	/// Produces a `409 Conflict` status.
	///
	/// [maps]: crate::maps
	#[track_caller]
	pub(crate) fn must_have_mappers() -> Self {
		Self::new(ErrorKind::MustHaveMappers)
	}

	/// An error that can occur when updating [maps].
	///
	/// Updating a map includes updating its courses. These updates are keyed by course ID. If
	/// the supplied course IDs don't belong to the map being updated, that's most likely a
	/// mistake by the client and should produce an error.
	///
	/// Produces a `409 Conflict` status.
	///
	/// [maps]: crate::maps
	#[track_caller]
	pub(crate) fn mismatching_map_course(course_id: CourseID, map_id: MapID) -> Self {
		Self::new(ErrorKind::MismatchingMapCourse { course_id, map_id })
	}

	/// An error that can occur when updating [maps].
	///
	/// Updating a map includes updating its courses, which includes updating filters. These
	/// updates are keyed by filter ID. If the supplied filter IDs don't belong to the course
	/// being updated, that's most likely a mistake by the client and should produce an error.
	///
	/// Produces a `409 Conflict` status.
	///
	/// [maps]: crate::maps
	#[track_caller]
	pub(crate) fn mismatching_course_filter(filter_id: FilterID, course_id: CourseID) -> Self {
		Self::new(ErrorKind::MismatchingCourseFilter {
			filter_id,
			course_id,
		})
	}

	/// An error that can occur when [unbanning] players.
	///
	/// Any given ban can only ever be reverted once. When an unban request is made for a ban
	/// that has already been reverted, that should produce an error.
	///
	/// Produces a `409 Conflict` status.
	///
	/// [unbanning]: crate::bans::handlers::by_id::delete
	#[track_caller]
	pub(crate) fn ban_already_reverted(ban_id: BanID, unban_id: UnbanID) -> Self {
		Self::new(ErrorKind::BanAlreadyReverted { ban_id, unban_id })
	}

	/// An error that can occur when submitting new CS2KZ plugin versions.
	///
	/// The API keeps track of all the versions, and if a new version is submitted that is
	/// older than the latest one, that's probably wrong.
	///
	/// Produces a `409 Conflict` status.
	#[track_caller]
	pub(crate) fn outdated_plugin_version(
		submitted: semver::Version,
		latest: semver::Version,
	) -> Self {
		Self::new(ErrorKind::OutdatedPluginVersion { submitted, latest })
	}

	/// A generic `500 Internal Server Error`.
	///
	/// This constructor is reserved for errors that _should not_ occur, but _may_ occur. If
	/// such an error is ever returned, that's a bug.
	#[track_caller]
	pub(crate) fn logic<T>(message: T) -> Self
	where
		T: Display,
	{
		Self::new(ErrorKind::Logic(message.to_string()))
	}

	/// An error for wrapping a [`jwt`] error.
	///
	/// If this error ever gets constructed, it's a bug.
	///
	/// Produces a `500 Internal Server Error` status.
	#[track_caller]
	pub(crate) fn encode_jwt(error: jwt::errors::Error) -> Self {
		Self::new(ErrorKind::Jwt(error))
	}

	/// An error that can occur when downloading something from the Steam Workshop.
	///
	/// This error can only occur if [`Config::workshop_artifacts_path`][config] is missing.
	/// The environment variable for that value is required when compiled with the `production`
	/// feature enabled. Because downloading Workshop files requires an external dependency,
	/// it's optional for local testing.
	///
	/// Produces a `500 Internal Server Error` status.
	///
	/// [config]: crate::Config::workshop_artifacts_path
	#[track_caller]
	#[cfg(not(feature = "production"))]
	pub(crate) fn missing_workshop_asset_dir() -> Self {
		Self::new(ErrorKind::MissingWorkshopAssetDirectory)
	}

	/// An error that can occur when downloading something from the Steam Workshop.
	///
	/// This error can only occur if [`Config::depot_downloader_path`][config] is missing.
	/// The environment variable for that value is required when compiled with the `production`
	/// feature enabled. Because downloading Workshop files requires an external dependency,
	/// it's optional for local testing.
	///
	/// Produces a `500 Internal Server Error` status.
	///
	/// [config]: crate::Config::depot_downloader_path
	#[track_caller]
	#[cfg(not(feature = "production"))]
	pub(crate) fn missing_depot_downloader() -> Self {
		Self::new(ErrorKind::MissingDepotDownloader)
	}

	/// An error that can occur when downloading something from the Steam Workshop.
	///
	/// Workshop downloads require an external dependency called [DepotDownloader].
	/// If that executable fails, it will produce an [`io::Error`], and constructing this error
	/// is considered a bug.
	///
	/// Produces a `500 Internal Server Error` status.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[track_caller]
	pub(crate) fn depot_downloader(source: io::Error) -> Self {
		Self::new(ErrorKind::DepotDownloader(source))
	}

	/// An error that can occur when downloading a map from the Steam Workshop.
	///
	/// After downloading, opening the map file might fail for various reasons.
	///
	/// Produces a `500 Internal Server Error` status.
	#[track_caller]
	pub(crate) fn open_map_file(source: io::Error) -> Self {
		Self::new(ErrorKind::OpenMapFile(source))
	}

	/// An error that can occur when calculating the checksum for a downloaded Workshop map.
	///
	/// Produces a `500 Internal Server Error` status.
	#[track_caller]
	pub(crate) fn checksum(source: io::Error) -> Self {
		Self::new(ErrorKind::Checksum(source))
	}

	/// An error that can occur when making HTTP requests to external APIs such as the Steam
	/// Web API.
	///
	/// Produces a `502 Bad Gateway` status.
	#[track_caller]
	pub(crate) fn external_api_call(source: reqwest::Error) -> Self {
		Self::new(ErrorKind::ExternalApiCall(source))
	}
}

impl IntoResponse for Error {
	#[track_caller]
	fn into_response(self) -> Response {
		use ErrorKind as E;

		let message = self.kind.to_string();
		let status = match self.kind {
			E::NoContent => StatusCode::NO_CONTENT,
			E::InvalidInput { .. } | E::Header(_) => StatusCode::BAD_REQUEST,
			E::Unauthorized
			| E::ExpiredAccessKey
			| E::MissingSessionID
			| E::InsufficientPermissions { .. }
			| E::MustBeServerOwner => StatusCode::UNAUTHORIZED,
			E::NotFound { .. } => StatusCode::NOT_FOUND,
			E::AlreadyExists { .. }
			| E::MustHaveMappers
			| E::MismatchingMapCourse { .. }
			| E::MismatchingCourseFilter { .. }
			| E::BanAlreadyReverted { .. }
			| E::OutdatedPluginVersion { .. } => StatusCode::CONFLICT,
			E::Logic(_)
			| E::Database(_)
			| E::Jwt(_)
			| E::Reqwest(_)
			| E::DepotDownloader(_)
			| E::OpenMapFile(_)
			| E::Checksum(_) => StatusCode::INTERNAL_SERVER_ERROR,

			#[cfg(not(feature = "production"))]
			E::MissingWorkshopAssetDirectory | E::MissingDepotDownloader => {
				StatusCode::INTERNAL_SERVER_ERROR
			}

			E::ExternalApiCall(_) => StatusCode::BAD_GATEWAY,
			E::Path(ref rej) => rej.status(),
		};

		if status == StatusCode::INTERNAL_SERVER_ERROR {
			tracing::error!(?self, "internal server error occurred");
		} else {
			tracing::debug! {
				location = %self.location,
				kind = ?self.kind,
				attachments = ?self.attachments,
				error_message = %message,
				"returning error from request handler"
			};
		}

		let mut json = json!({ "message": message });

		#[allow(clippy::indexing_slicing)]
		if !self.attachments.is_empty() {
			json["debug_info"] = self
				.attachments
				.iter()
				.rev()
				.map(|attachment| format!("{attachment}"))
				.collect_vec()
				.into();
		}

		(status, Json(json)).into_response()
	}
}

impl From<sqlx::Error> for Error {
	#[track_caller]
	fn from(error: sqlx::Error) -> Self {
		use sqlx::Error as E;

		match error {
			error @ (E::Configuration(_) | E::Tls(_) | E::AnyDriverError(_) | E::Migrate(_)) => {
				unreachable!("these do not happen after initial setup ({error})");
			}
			error => Self::new(error),
		}
	}
}

impl From<reqwest::Error> for Error {
	#[track_caller]
	fn from(error: reqwest::Error) -> Self {
		if matches!(error.status(), Some(status) if status.is_server_error()) {
			Self::new(ErrorKind::ExternalApiCall(error))
		} else {
			Self::new(ErrorKind::Reqwest(error))
		}
	}
}

impl From<TypedHeaderRejection> for Error {
	#[track_caller]
	fn from(rejection: TypedHeaderRejection) -> Self {
		Self::new(rejection)
	}
}

impl From<PathRejection> for Error {
	#[track_caller]
	fn from(rejection: PathRejection) -> Self {
		Self::new(rejection)
	}
}

impl<E> From<ConvertIDError<E>> for Error
where
	E: std::error::Error + Send + Sync + 'static,
{
	#[track_caller]
	fn from(error: ConvertIDError<E>) -> Self {
		Self::logic("failed to convert a raw database id").context(error)
	}
}
