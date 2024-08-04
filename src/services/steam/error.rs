//! The errors that can occur when interacting with this service.

use std::io;

use thiserror::Error;

use super::openid::OpenIDRejection;
use super::WorkshopID;
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We failed to extract an OpenID payload from a request.
	#[error(transparent)]
	ExtractOpenIDPayload(#[from] OpenIDRejection),

	/// Steam's API returned an error when we tried to fetch information about a
	/// map.
	#[error("invalid workshop ID")]
	InvalidWorkshopID
	{
		/// The workshop ID of the map we tried to fetch.
		workshop_id: WorkshopID,
	},

	/// Steam's API returned something that wasn't a map when we tried to fetch
	/// the `workshop_id`.
	#[error("workshop ID does not belong to a map")]
	NotAMap
	{
		/// The workshop ID of the map we tried to fetch.
		workshop_id: WorkshopID,
	},

	/// We failed to download a workshop map.
	#[error("failed to download workshop map")]
	DownloadWorkshopMap(#[from] io::Error),

	/// We failed to make an HTTP request to Steam's Web API.
	#[error("failed to make http request")]
	Http(#[from] reqwest::Error),
}

impl IntoProblemDetails for Error
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::ExtractOpenIDPayload(source) => source.problem_type(),
			Self::InvalidWorkshopID { .. } => ProblemType::ResourceNotFound,
			Self::NotAMap { .. } => ProblemType::WorkshopItemNotAMap,
			Self::DownloadWorkshopMap(_) => ProblemType::DownloadWorkshopMap,
			Self::Http(source) => source.problem_type(),
		}
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		match self {
			Self::ExtractOpenIDPayload(source) => {
				source.add_extension_members(ext);
			}
			Self::InvalidWorkshopID { workshop_id } | Self::NotAMap { workshop_id } => {
				ext.add("workshop_id", workshop_id);
			}
			_ => {}
		}
	}
}
