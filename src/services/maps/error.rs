//! The errors that can occur when interacting with this service.

use std::io;

use thiserror::Error;

use super::{CourseID, FilterID, MapID};
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::services::steam;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We have no data to return.
	#[error("no data")]
	NoData,

	/// A request dedicated to a specific map was made, but the map could not be
	/// found.
	#[error("map does not exist")]
	MapDoesNotExist,

	/// A request involving a specific mapper was made, but the mapper could not
	/// be found.
	#[error("one of the submitted mappers is unknown")]
	MapperDoesNotExist,

	/// A request wanted to create a new map / remove mappers from an existing
	/// map, but specified all the mappers associated with that map.
	///
	/// Every map must have at least one mapper at any given time.
	#[error("every map must have at least one mapper")]
	MapMustHaveMappers,

	/// A request wanted to create a new map but didn't submit any courses.
	///
	/// Every map must have at least one course at any given time.
	#[error("every map must have at least one course")]
	MapMustHaveCourses,

	/// A request wanted to create a new map or update an existing one, in which
	/// mappers were supposed to be removed from a course.
	///
	/// In the case of a new map, no mappers were included in the course.
	/// In the case of an update, all mappers associated with that course were
	/// supposed to be removed.
	///
	/// Every course must have at least one mapper at any given time.
	#[error("every course must have at least one mapper")]
	CourseMustHaveMappers
	{
		/// This is `None` for new map submissions, when the new courses don't
		/// have IDs yet.
		///
		/// If this error was caused by an update, this will be `Some` and
		/// contain the ID of the course whose mappers were supposed to be
		/// removed.
		course_id: Option<CourseID>,
	},

	/// A request wanted to update a map's courses, but specified a course ID
	/// that does not belong to the map ID it made the request for.
	#[error("course is not part of map")]
	MismatchingCourseID
	{
		/// The ID of the map the update request was made for.
		map_id: MapID,

		/// The ID of the course that was supposed to be updated, but does not
		/// belong to the map.
		course_id: CourseID,
	},

	/// A request wanted to update a course's filters, but specified a filter ID
	/// that does not belong to the course ID it made the request for.
	#[error("filter is not part of course")]
	MismatchingFilterID
	{
		/// The ID of the course the update belonged to.
		course_id: CourseID,

		/// The ID of the filter that was supposed to be updated, but does not
		/// belong to the course.
		filter_id: FilterID,
	},

	/// An operation using the steam service failed.
	#[error(transparent)]
	Steam(#[from] steam::Error),

	/// An I/O error occurred while calculating a map's checksum.
	#[error("failed to calculate map checksum")]
	CalculateMapChecksum(#[from] io::Error),

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),
}

impl IntoProblemDetails for Error
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::NoData => ProblemType::NoContent,
			Self::MapDoesNotExist => ProblemType::ResourceNotFound,
			Self::MapMustHaveMappers | Self::CourseMustHaveMappers { .. } => {
				ProblemType::MustHaveMappers
			}
			Self::MapMustHaveCourses => ProblemType::MapMustHaveCourses,
			Self::MismatchingCourseID { .. } | Self::MismatchingFilterID { .. } => {
				ProblemType::UnrelatedUpdate
			}
			Self::MapperDoesNotExist => ProblemType::ResourceNotFound,
			Self::Steam(source) => source.problem_type(),
			Self::CalculateMapChecksum(_) => ProblemType::Internal,
			Self::Database(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		match self {
			Self::CourseMustHaveMappers { course_id: Some(course_id) } => {
				ext.add("course_id", course_id);
			}
			Self::MismatchingCourseID { map_id, course_id } => {
				ext.add("map_id", map_id);
				ext.add("course_id", course_id);
			}
			Self::MismatchingFilterID { course_id, filter_id } => {
				ext.add("course_id", course_id);
				ext.add("filter_id", filter_id);
			}
			Self::Steam(source) => {
				source.add_extension_members(ext);
			}
			_ => {}
		}
	}
}
