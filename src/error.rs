//! The main error type used across the code base.
//!
//! [`Error`] implements [`IntoResponse`], so that it can be returned by handlers.
//! Most fallible functions in this crate return [`Result<T>`].
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
use tracing::{debug, error};

use crate::authorization::Permissions;
use crate::bans::{BanID, UnbanID};
use crate::make_id::ConvertIDError;
use crate::maps::{CourseID, FilterID, MapID};

/// Convenience type alias, because this type is long.
type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Convenient type alias to use for fallible functions.
///
/// All fallible functions in this crate return an [`Error`] in their failure case, so spelling it
/// out 500 times is not desirable.
///
/// [`Error`]: struct@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The main error type used in this crate.
///
/// Every fallible function returns it.
#[derive(Debug, Error)]
pub struct Error {
	/// The kind of error that occurred.
	kind: ErrorKind,

	/// Source code location of where the error occurred.
	location: Location<'static>,

	/// A list of 'attachments'.
	///
	/// These can provide additional context when debugging in the form of errors or messages.
	attachments: Vec<Attachment>,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let Self {
			kind,
			location,
			attachments,
		} = self;

		write!(
			f,
			"[{}:{}:{}] {}",
			location.file(),
			location.line(),
			location.column(),
			kind
		)?;

		if !attachments.is_empty() {
			write!(f, ":")?;

			for attachment in attachments.iter().rev() {
				write!(f, "\n  - {attachment}")?;
			}
		}

		Ok(())
	}
}

/// Public facing error message for a 400.
const UNAUTHORIZED_MSG: &str = "you are not permitted to perform this action";

/// The different kinds of errors that can occur anywhere across the code base.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, Error)]
enum ErrorKind {
	#[error("no content")]
	NoContent,

	#[error("unknown {what}")]
	UnknownInput { what: &'static str },

	#[error("invalid {what}")]
	InvalidInput { what: &'static str },

	#[error("{UNAUTHORIZED_MSG}")]
	Unauthorized,

	#[error("{UNAUTHORIZED_MSG}")]
	InvalidCS2RefreshKey,

	#[error("this access key is expired; request a new one")]
	ExpiredCS2AccessKey,

	#[error("you are not logged in")]
	MissingSessionID,

	#[error("you are not logged in")]
	InvalidSessionID,

	#[error("{UNAUTHORIZED_MSG}")]
	InsufficientPermissions,

	#[error("{UNAUTHORIZED_MSG}")]
	MustBeServerOwner,

	#[error("{UNAUTHORIZED_MSG}")]
	InvalidApiKey,

	#[error("{UNAUTHORIZED_MSG}")]
	ExpiredApiKey,

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

	#[error("logic assertion failed: {0}")]
	Logic(String),

	#[error("database error")]
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

/// An error attachment.
///
/// This is ad-hoc context attached to an [`Error`] via [`Error::context()`].
///
/// [`Error`]: struct@Error
#[derive(Debug, Display)]
#[display("'{}' at {}:{}:{}", context, location.file(), location.line(), location.column())]
struct Attachment {
	/// The context itself.
	context: BoxedError,

	/// The source location of where this context was attached.
	location: Location<'static>,
}

impl Attachment {
	/// Creates a new attachment from the given `context` using the caller's [`Location`].
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
	/// Create a new error.
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

	/// Create a new [`204 No Content`][no-content] error.
	///
	/// [no-content]: ErrorKind::NoContent
	#[track_caller]
	pub(crate) fn no_content() -> Self {
		Self::new(ErrorKind::NoContent)
	}

	/// Create a new error signaling "unknown" user input, such as a non-existent ID.
	#[track_caller]
	pub(crate) fn unknown(what: &'static str) -> Self {
		Self::new(ErrorKind::UnknownInput { what })
	}

	/// Create a new error signaling that user input was invalid.
	///
	/// This could be because it could not be parsed.
	#[track_caller]
	pub(crate) fn invalid(what: &'static str) -> Self {
		Self::new(ErrorKind::InvalidInput { what })
	}

	/// A generic "unauthorized" error.
	///
	/// Use sparingly.
	#[track_caller]
	pub(crate) fn unauthorized() -> Self {
		Self::new(ErrorKind::Unauthorized)
	}

	/// An error that is produced during [CS2 server authentication].
	///
	/// [CS2 server authentication]: crate::servers::handlers::key::generate_temp
	#[track_caller]
	pub(crate) fn invalid_cs2_refresh_key() -> Self {
		Self::new(ErrorKind::InvalidCS2RefreshKey)
	}

	/// An error that is produced during [CS2 server authentication].
	///
	/// [CS2 server authentication]: <crate::authentication::Jwt as axum::extract::FromRequestParts>::from_request_parts
	#[track_caller]
	pub(crate) fn expired_cs2_access_key() -> Self {
		Self::new(ErrorKind::ExpiredCS2AccessKey)
	}

	/// An error that is produced during [session authentication].
	///
	/// [session authentication]: crate::authentication::session
	#[track_caller]
	pub(crate) fn missing_session_id() -> Self {
		Self::new(ErrorKind::MissingSessionID)
	}

	/// An error that is produced during [session authentication].
	///
	/// [session authentication]: crate::authentication::session
	#[track_caller]
	pub(crate) fn invalid_session_id() -> Self {
		Self::new(ErrorKind::InvalidSessionID)
	}

	/// An error that is produced during [session authorization].
	///
	/// [session authentication]: crate::authorization::has_permissions
	#[track_caller]
	pub(crate) fn insufficient_permissions(required_permissions: Permissions) -> Self {
		Self::new(ErrorKind::InsufficientPermissions)
			.context(format!("required permissions: {required_permissions}"))
	}

	/// An error that is produced during [session authorization].
	///
	/// [session authentication]: crate::authorization::is_server_admin_or_owner
	#[track_caller]
	pub(crate) fn must_be_server_owner() -> Self {
		Self::new(ErrorKind::MustBeServerOwner)
	}

	/// An error that is produced during [API key authentication].
	///
	/// [API key authentication]: crate::authentication::api_key
	#[track_caller]
	pub(crate) fn invalid_api_key() -> Self {
		Self::new(ErrorKind::InvalidApiKey)
	}

	/// An error that is produced during [API key authentication].
	///
	/// [API key authentication]: crate::authentication::api_key
	#[track_caller]
	pub(crate) fn expired_api_key() -> Self {
		Self::new(ErrorKind::ExpiredApiKey)
	}

	/// Create a new error signaling that some submitted data already exists.
	#[track_caller]
	pub(crate) fn already_exists(what: &'static str) -> Self {
		Self::new(ErrorKind::AlreadyExists { what })
	}

	/// When submitting new maps / updating existing maps, we need to ensure that a given map
	/// or course always has at least 1 mapper.
	#[track_caller]
	pub(crate) fn must_have_mappers() -> Self {
		Self::new(ErrorKind::MustHaveMappers)
	}

	/// An error that can occur when updating map courses.
	///
	/// These updates are provided as maps from `course_id -> data` per map, so the ID might
	/// not belong to the map being updated.
	#[track_caller]
	pub(crate) fn mismatching_map_course(course_id: CourseID, map_id: MapID) -> Self {
		Self::new(ErrorKind::MismatchingMapCourse { course_id, map_id })
	}

	/// An error that can occur when updating course filters.
	///
	/// These updates are provided as maps from `filter_id -> data` per course, so the ID might
	/// not belong to the course being updated.
	#[track_caller]
	pub(crate) fn mismatching_course_filter(filter_id: FilterID, course_id: CourseID) -> Self {
		Self::new(ErrorKind::MismatchingCourseFilter {
			filter_id,
			course_id,
		})
	}

	/// An error that is produced when [reverting a ban].
	///
	/// Every ban can only be reverted once, so if there are duplicate requests for a given
	/// ban, they should fail.
	///
	/// [reverting a ban]: crate::bans::handlers::by_id::delete
	#[track_caller]
	pub(crate) fn ban_already_reverted(ban_id: BanID, unban_id: UnbanID) -> Self {
		Self::new(ErrorKind::BanAlreadyReverted { ban_id, unban_id })
	}

	/// Create an error signaling some internal logic error.
	///
	/// You can think of this like a "soft" assertion error.
	#[track_caller]
	pub(crate) fn logic<T>(message: T) -> Self
	where
		T: Display,
	{
		Self::new(ErrorKind::Logic(message.to_string()))
	}

	/// An error occurred when encoding a JWT.
	#[track_caller]
	pub(crate) fn encode_jwt(error: jwt::errors::Error) -> Self {
		Self::new(ErrorKind::Jwt(error))
	}

	/// When downloading a [workshop map][map], we need to store it somewhere.
	///
	/// Where we store it is configured through an environment variable, which is allowed to
	/// not exist (for local testing purposes).
	///
	/// [map]: crate::steam::workshop::MapFile
	#[track_caller]
	#[cfg(not(feature = "production"))]
	pub(crate) fn missing_workshop_asset_dir() -> Self {
		Self::new(ErrorKind::MissingWorkshopAssetDirectory)
	}

	/// For downloading a [workshop map][map], we use a program called [DepotDownloader].
	///
	/// The path to its executable is configured through environment variables, which is
	/// allowed to not exist (for local testing purposes).
	///
	/// [map]: crate::steam::workshop::MapFile
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[track_caller]
	#[cfg(not(feature = "production"))]
	pub(crate) fn missing_depot_downloader() -> Self {
		Self::new(ErrorKind::MissingDepotDownloader)
	}

	/// For downloading a [workshop map][map], we use a program called [DepotDownloader].
	///
	/// Running this program might fail at various points.
	///
	/// See [`MapFile::download()`][download] for more information.
	///
	/// [map]: crate::steam::workshop::MapFile
	/// [download]: crate::steam::workshop::MapFile::download
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[track_caller]
	pub(crate) fn depot_downloader(source: io::Error) -> Self {
		Self::new(ErrorKind::DepotDownloader(source))
	}

	/// After downloading a [workshop map][map], we [hash its file contents using crc32][hash].
	///
	/// But before we can do that, we need to open the file!
	///
	/// [map]: crate::steam::workshop::MapFile
	/// [hash]: crate::steam::workshop::MapFile::checksum
	#[track_caller]
	pub(crate) fn open_map_file(source: io::Error) -> Self {
		Self::new(ErrorKind::OpenMapFile(source))
	}

	/// After downloading a [workshop map][map], we [hash its file contents using crc32][hash].
	///
	/// This operation might fail when reading from the file.
	///
	/// [map]: crate::steam::workshop::MapFile
	/// [hash]: crate::steam::workshop::MapFile::checksum
	#[track_caller]
	pub(crate) fn checksum(source: io::Error) -> Self {
		Self::new(ErrorKind::Checksum(source))
	}

	/// We make requests to other APIs, such as Steam's Web API.
	#[track_caller]
	pub(crate) fn external_api_call(source: reqwest::Error) -> Self {
		Self::new(ErrorKind::ExternalApiCall(source))
	}

	/// Attach additional context to this error.
	///
	/// This could be another error type, or simply a message.
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
}

impl IntoResponse for Error {
	#[allow(clippy::indexing_slicing)]
	fn into_response(self) -> Response {
		use ErrorKind as E;

		let Self {
			kind,
			location,
			attachments,
		} = self;

		let message = kind.to_string();
		let status = match kind {
			E::NoContent => StatusCode::NO_CONTENT,
			E::UnknownInput { .. } | E::InvalidInput { .. } | E::Header(_) => {
				StatusCode::BAD_REQUEST
			}
			E::Unauthorized
			| E::InvalidCS2RefreshKey
			| E::ExpiredCS2AccessKey
			| E::MissingSessionID
			| E::InvalidSessionID
			| E::InsufficientPermissions
			| E::MustBeServerOwner
			| E::InvalidApiKey
			| E::ExpiredApiKey => StatusCode::UNAUTHORIZED,
			E::AlreadyExists { .. }
			| E::MustHaveMappers
			| E::MismatchingMapCourse { .. }
			| E::MismatchingCourseFilter { .. }
			| E::BanAlreadyReverted { .. } => StatusCode::CONFLICT,
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

		debug! {
			%location,
			%status,
			%message,
			?kind,
			?attachments,
			"error occurred in request handler",
		};

		let mut json = json!({ "message": message });

		if !attachments.is_empty() {
			json["debug_info"] = attachments
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
			E::Configuration(_) | E::Tls(_) | E::AnyDriverError(_) | E::Migrate(_) => {
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
