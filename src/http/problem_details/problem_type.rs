//! This module contains the [`ProblemType`] enum.
//!
//! It represents an exhaustive list of all the possible error conditions the
//! API might return.

use std::sync::OnceLock;

use serde::{Serialize, Serializer};
use tap::Tap;
use url::Url;

/// The base URL for the problem type documentation.
static BASE_URL: OnceLock<Url> = OnceLock::new();

/// Sets `URL`.
#[doc(hidden)]
pub(crate) fn set_base_url(url: Url)
{
	assert!(BASE_URL.set(url).is_ok(), "called `set_base_url()` twice!");
}

/// A problem type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, cs2kz_api_macros::ProblemType)]
pub enum ProblemType
{
	/// An endpoint which can return many results has no results to return for a
	/// given request.
	#[status = 204]
	NoContent,

	/// You failed to provide a required request header.
	#[status = 400]
	MissingHeader,

	/// You failed to provide a required path parameter.
	#[status = 400]
	MissingPathParameters,

	/// You did not provide the necessary authentication/authorization
	/// information to perform your request.
	#[status = 401]
	Unauthorized,

	/// During the OpenID authentication flow, the API's callback route was hit
	/// with a payload that could not be verified by Steam.
	#[status = 401]
	InvalidOpenIDPayload,

	/// A requested resource could not be found.
	#[status = 404]
	ResourceNotFound,

	/// A request for creating a resource was made, but rejected because the
	/// resource already exists.
	#[status = 409]
	ResourceAlreadyExists,

	/// Maps and Courses must have at least 1 mapper at any given time.
	///
	/// When making a request to update a map, you can specify a list of mapper
	/// IDs to remove from the map or its courses. If the map/course would have
	/// 0 mappers after the deletion, this error is returned instead of
	/// applying the update.
	#[status = 409]
	MustHaveMappers,

	/// Maps must have at least 1 course at any given time.
	///
	/// When submitting a new map, you also submit a list of courses for that
	/// map. That list cannot be empty.
	#[status = 409]
	MapMustHaveCourses,

	/// When updating (parts of) a resource, such as a map, you may be able to
	/// supply pairs of resource IDs and update payloads. For example, when
	/// updating a map, you can supply a list of course updates. These are
	/// mappings from course ID -> update payload. However, if the provided
	/// course ID does not "belong" to the map you're trying to update, then
	/// you probably made a mistake, and the request is rejected.
	#[status = 409]
	UnrelatedUpdate,

	/// An action you tried to perform could only be performed once, and has
	/// already been performed in the past.
	#[status = 409]
	ActionAlreadyPerformed,

	/// You provided a timestamp that did not make sense.
	///
	/// For example, when providing an expiration date, it cannot be before the
	/// creation date of the same resource.
	#[status = 409]
	IllogicalTimestamp,

	/// You requested to perform an update on a resource, but the update you
	/// provided did not actually change anything about the resource. This was
	/// likely a logic error on your part.
	#[status = 409]
	NoChange,

	/// You submitted a new version of some resource, but the latest version of
	/// that resource is newer than what you provided. This was likely a logic
	/// error on your part.
	#[status = 409]
	OutdatedVersion,

	/// You requested to create/update a map, and in the process the server
	/// attempted to fetch the map from Steam's workshop. The response it got
	/// back did not have the expected shape though, so we assume it was some
	/// other item, and not a map.
	#[status = 409]
	WorkshopItemNotAMap,

	/// You provided path parameters which could not be deserialized.
	#[status = 422]
	InvalidPathParameters,

	/// You provided a query string which could not be deserialized.
	#[status = 422]
	InvalidQueryString,

	/// You provided a request header could not be deserialized.
	#[status = 422]
	InvalidHeader,

	/// You provided a request body which could not be deserialized.
	#[status = 422]
	InvalidRequestBody,

	/// We made a request to an external service and failed to decode the
	/// response.
	#[status = 500]
	DecodeExternal,

	/// We tried downloading a CS2 map from the Steam Workshop, but it went
	/// wrong somehow.
	#[status = 500]
	DownloadWorkshopMap,

	/// An internal failure occurred.
	///
	/// Any occurrence of this problem type is considered a bug!
	#[status = 500]
	Internal,

	/// We failed to communicate with an external service, such as Steam.
	#[status = 502]
	ExternalService,
}

impl Serialize for ProblemType
{
	// Serialize as a URI as specified by the RFC.
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		BASE_URL
			.get()
			.cloned()
			.unwrap_or_else(|| "https://api.cs2kz.org".parse::<Url>().expect("valid url"))
			.join("/docs/problem-types")
			.expect("valid url")
			.tap_mut(|url| url.set_fragment(Some(self.slug())))
			.serialize(serializer)
	}
}
