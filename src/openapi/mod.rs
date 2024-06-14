//! Everything related to [OpenAPI].
//!
//! This project uses the [`utoipa`] crate for generating an OpenAPI specification from code.
//! The [`Spec`] struct in this module lists out all the relevant types, routes, and other metadata
//! that will be included in the spec.
//!
//! [OpenAPI]: https://spec.openapis.org/oas/latest.html

use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::openapi::security::Security;

pub mod parameters;
pub mod responses;
pub mod security;

#[derive(Debug, Clone, Deref, DerefMut, OpenApi)]
#[openapi(
  info(
    title = "CS2KZ API",
    description = "Source Code available on [GitHub](https://github.com/KZGlobalTeam/cs2kz-api).",
    license(
      name = "Licensed under the GPLv3",
      url = "https://www.gnu.org/licenses/gpl-3.0",
    ),
  ),
  modifiers(&Security),
  paths(
    crate::players::handlers::root::get,
    crate::players::handlers::root::post,
    crate::players::handlers::by_identifier::get,
    crate::players::handlers::by_identifier::patch,
    crate::players::handlers::steam::get,
    crate::players::handlers::preferences::get,

    crate::maps::handlers::root::get,
    crate::maps::handlers::root::put,
    crate::maps::handlers::by_identifier::get,
    crate::maps::handlers::by_identifier::patch,

    crate::servers::handlers::root::get,
    crate::servers::handlers::root::post,
    crate::servers::handlers::by_identifier::get,
    crate::servers::handlers::by_identifier::patch,
    crate::servers::handlers::key::generate_temp,
    crate::servers::handlers::key::put_perma,
    crate::servers::handlers::key::delete_perma,

    crate::jumpstats::handlers::root::get,
    crate::jumpstats::handlers::root::post,
    crate::jumpstats::handlers::by_id::get,
    crate::jumpstats::handlers::replays::get,

    crate::records::handlers::root::get,
    crate::records::handlers::root::post,
    crate::records::handlers::top::get,
    crate::records::handlers::by_id::get,
    crate::records::handlers::replays::get,

    crate::bans::handlers::root::get,
    crate::bans::handlers::root::post,
    crate::bans::handlers::by_id::get,
    crate::bans::handlers::by_id::patch,
    crate::bans::handlers::by_id::delete,

    crate::game_sessions::handlers::by_id::get,

    crate::authentication::handlers::login,
    crate::authentication::handlers::logout,
    crate::authentication::handlers::callback,

    crate::admins::handlers::root::get,
    crate::admins::handlers::by_id::get,
    crate::admins::handlers::by_id::put,

    crate::plugin::handlers::versions::get,
    crate::plugin::handlers::versions::post,
  ),
  components(
    schemas(
      cs2kz::SteamID,
      cs2kz::Mode,
      cs2kz::Style,
      cs2kz::Tier,
      cs2kz::JumpType,
      cs2kz::PlayerIdentifier,
      cs2kz::MapIdentifier,
      cs2kz::CourseIdentifier,
      cs2kz::ServerIdentifier,
      cs2kz::GlobalStatus,
      cs2kz::RankedStatus,

      crate::openapi::parameters::Offset,
      crate::openapi::parameters::Limit,
      crate::openapi::parameters::SortingOrder,
      crate::openapi::responses::Object,

      crate::time::Seconds,

      crate::steam::workshop::WorkshopID,

      crate::players::Player,
      crate::players::NewPlayer,
      crate::players::PlayerUpdate,

      crate::maps::FullMap,
      crate::maps::MapID,
      crate::maps::Course,
      crate::maps::CourseID,
      crate::maps::Filter,
      crate::maps::FilterID,
      crate::maps::NewMap,
      crate::maps::NewCourse,
      crate::maps::NewFilter,
      crate::maps::CreatedMap,
      crate::maps::MapUpdate,
      crate::maps::CourseUpdate,
      crate::maps::FilterUpdate,
      crate::maps::MapInfo,
      crate::maps::CourseInfo,

      crate::servers::Server,
      crate::servers::ServerID,
      crate::servers::NewServer,
      crate::servers::CreatedServer,
      crate::servers::ServerUpdate,
      crate::servers::AccessKeyRequest,
      crate::servers::RefreshKey,
      crate::servers::ServerInfo,

      crate::jumpstats::Jumpstat,
      crate::jumpstats::JumpstatID,
      crate::jumpstats::NewJumpstat,
      crate::jumpstats::CreatedJumpstat,

      crate::records::Record,
      crate::records::RecordID,
      crate::records::BhopStats,
      crate::records::NewRecord,
      crate::records::CreatedRecord,
      crate::records::handlers::root::SortRecordsBy,

      crate::bans::Ban,
      crate::bans::BanID,
      crate::bans::BanReason,
      crate::bans::Unban,
      crate::bans::UnbanID,
      crate::bans::NewBan,
      crate::bans::CreatedBan,
      crate::bans::BanUpdate,
      crate::bans::NewUnban,
      crate::bans::CreatedUnban,

      crate::game_sessions::GameSession,
      crate::game_sessions::GameSessionID,
      crate::game_sessions::TimeSpent,

      crate::admins::Admin,
      crate::admins::AdminUpdate,

      crate::plugin::PluginVersion,
      crate::plugin::PluginVersionID,
      crate::plugin::NewPluginVersion,
      crate::plugin::CreatedPluginVersion,
    ),
  ),
)]
#[allow(missing_docs)]
pub struct Spec(utoipa::openapi::OpenApi);

impl Spec {
	/// Creates a new [`Spec`].
	pub fn new() -> Self {
		Self(Self::openapi())
	}

	/// Returns an iterator over the registered API routes and their allowed HTTP methods.
	pub fn routes(&self) -> impl Iterator<Item = (&str, String)> {
		self.paths.paths.iter().map(|(path, handler)| {
			let methods = handler
				.operations
				.keys()
				.map(|method| format!("{method:?}").to_uppercase())
				.join(", ");

			(path.as_str(), methods)
		})
	}

	/// Generates a JSON representation of this OpenAPI spec.
	pub fn as_json(&self) -> String {
		self.to_pretty_json().expect("spec is valid")
	}

	/// Creates a [`SwaggerUi`], which can be turned into an [`axum::Router`], that will serve
	/// a SwaggerUI web page and a JSON file representing this OpenAPI spec.
	pub fn swagger_ui(self) -> SwaggerUi {
		SwaggerUi::new("/docs/swagger-ui").url("/docs/openapi.json", self.0)
	}
}
