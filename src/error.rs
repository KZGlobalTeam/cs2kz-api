use std::io;
use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use cs2kz::Mode;
use serde_json::json;
use thiserror::Error as ThisError;
use tracing::{error, warn};

use crate::{state, steam};

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

	#[error("No data available for the given query.")]
	NoContent,

	#[error("You do not have access to this resource.")]
	Unauthorized,

	#[error("You do not have access to this resource.")]
	Forbidden,

	#[error("Your token is expired.")]
	ExpiredToken,
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let json = json!({ "message": self.to_string() });
		let code = match self {
			Error::IO(err) => {
				error!(%err, "encountered I/O error at runtime");
				StatusCode::INTERNAL_SERVER_ERROR
			}
			Error::State(err) => {
				unreachable!("{err}");
			}
			Error::MySql(err) => {
				error!(%err, "database error");
				StatusCode::INTERNAL_SERVER_ERROR
			}
			Error::Steam(err) => {
				warn!(%err, "steam error");
				StatusCode::BAD_GATEWAY
			}
			Error::NoMappers
			| Error::NonContiguousStages
			| Error::NoCourseMappers { .. }
			| Error::InvalidFilterAmount { .. }
			| Error::MissingFilter { .. }
			| Error::UnrankableFilter { .. }
			| Error::UnrankableFilterWithID { .. }
			| Error::UnknownMapID(_)
			| Error::InvalidCourse { .. }
			| Error::InvalidFilter { .. } => StatusCode::BAD_REQUEST,
			Error::ForeignHost => StatusCode::UNAUTHORIZED,
			Error::NoContent => StatusCode::NO_CONTENT,
			Error::Unauthorized => StatusCode::UNAUTHORIZED,
			Error::Forbidden | Error::ExpiredToken => StatusCode::FORBIDDEN,
		};

		(code, Json(json)).into_response()
	}
}
