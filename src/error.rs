use std::io;
use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::typed_header::TypedHeaderRejection;
use cs2kz::{Mode, SteamID};
use serde_json::json;
use thiserror::Error as ThisError;

use crate::{audit, middleware, state, steam};

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr while the API is running.
#[derive(Debug, ThisError)]
pub enum Error {
	#[error("{0}")]
	IO(#[from] io::Error),

	#[error("Failed to initialize API state: {0}")]
	State(#[from] state::Error),

	#[error("Database Error: {0}")]
	MySql(#[from] sqlx::Error),

	#[error("{0}")]
	Steam(#[from] steam::Error),

	#[error("There must be at least 1 mapper per map.")]
	NoMappers,

	#[error("There must be at least 1 course per map.")]
	NoCourses,

	#[error("Stages must start at 1 and cannot exceed 100.")]
	InvalidStage,

	#[error("Course stages must be contiguous.")]
	NonContiguousStages,

	#[error(
		"There must be at least 1 mapper per course. Course with stage `{stage}` has no mappers."
	)]
	NoCourseMappers { stage: u8 },

	#[error("Every course must have 4 filters. Course with stage `{stage}` has {amount} filters.")]
	InvalidFilterAmount { stage: u8, amount: usize },

	#[error(
		"Every course must have 4 unique filters. Course with stage `{stage}` is missing a filter for ({mode}, {teleports})."
	)]
	MissingFilter {
		stage: u8,
		mode: Mode,
		teleports: bool,
	},

	#[error(
		"Course with stage `{stage}` and filter ({mode}, {teleports}) cannot be ranked, because it is too difficult."
	)]
	UnrankableFilter {
		stage: u8,
		mode: Mode,
		teleports: bool,
	},

	#[error("Filter with ID `{id}` cannot be above T8 and ranked at the same time.")]
	UnrankableFilterWithID { id: u32 },

	#[error("There is no map with ID `{0}`.")]
	UnknownMapID(u16),

	#[error("Course `{course_id}` is not part of map `{map_id}`.")]
	InvalidCourse { map_id: u16, course_id: u32 },

	#[error("Filter `{filter_id}` is not part of course `{course_id}`.")]
	InvalidFilter { course_id: u32, filter_id: u32 },

	#[error("`return_to` host does not match the API's public URL.")]
	ForeignHost,

	#[error("{0}")]
	Header(#[from] TypedHeaderRejection),

	#[error("No data available for the given query.")]
	NoContent,

	#[error("You do not have access to this resource.")]
	Unauthorized,

	#[error("You do not have access to this resource.")]
	Forbidden,

	#[error("{0}")]
	Middleware(#[from] middleware::Error),

	#[error("Player `{steam_id}` is not in the database.")]
	UnknownPlayer { steam_id: SteamID },

	#[error("One of the submitted mappers is unknown to the database.")]
	UnknownMapper,

	#[error("Player with SteamID `{steam_id}` already exists.")]
	PlayerAlreadyExists { steam_id: SteamID },

	#[error("Invalid Ban ID `{0}`.")]
	InvalidBanID(u32),

	#[error("Invalid Server ID `{0}`.")]
	InvalidServerID(u16),

	#[error("Server `{server_id}` has an invalid plugin version ({plugin_version_id}).")]
	InvalidPluginVersion {
		server_id: u16,
		plugin_version_id: u16,
	},

	#[error("Invalid Service ID `{0}`.")]
	InvalidServiceID(u64),
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let json = json!({ "message": self.to_string() });
		let code = match self {
			Error::IO(err) => {
				audit!(error, "encountered I/O error at runtime", %err);
				StatusCode::INTERNAL_SERVER_ERROR
			}
			Error::State(err) => {
				unreachable!("{err}");
			}
			Error::MySql(err) => {
				audit!(error, "database error", %err);
				StatusCode::INTERNAL_SERVER_ERROR
			}
			Error::Steam(err) => {
				audit!(warn, "steam error", %err);
				StatusCode::BAD_GATEWAY
			}
			Error::NoMappers
			| Error::NoCourses
			| Error::InvalidStage
			| Error::NonContiguousStages
			| Error::NoCourseMappers { .. }
			| Error::InvalidFilterAmount { .. }
			| Error::MissingFilter { .. }
			| Error::UnrankableFilter { .. }
			| Error::UnrankableFilterWithID { .. }
			| Error::UnknownMapID(_)
			| Error::InvalidCourse { .. }
			| Error::InvalidFilter { .. }
			| Error::UnknownPlayer { .. }
			| Error::UnknownMapper
			| Error::PlayerAlreadyExists { .. }
			| Error::InvalidBanID(_)
			| Error::InvalidServerID(_)
			| Error::InvalidPluginVersion { .. }
			| Error::InvalidServiceID(_)
			| Error::Header(_) => StatusCode::BAD_REQUEST,
			Error::NoContent => StatusCode::NO_CONTENT,
			Error::Unauthorized | Error::ForeignHost => StatusCode::UNAUTHORIZED,
			Error::Forbidden => StatusCode::FORBIDDEN,
			Error::Middleware(error) => {
				return error.into_response();
			}
		};

		(code, Json(json)).into_response()
	}
}
