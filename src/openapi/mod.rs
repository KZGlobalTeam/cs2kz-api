//! [OpenAPI] specification for the API.
//!
//! [OpenAPI]: https://www.openapis.org

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::services;

mod security;
pub use security::Security;

pub mod responses;

/// The API's OpenAPI schema.
#[rustfmt::skip]
#[utoipauto::utoipauto(paths = "./src/services/health, ./src/services/players, ./src/services/maps, ./src/services/servers, ./src/services/records, ./src/services/jumpstats, ./src/services/bans, ./src/services/admins, ./src/services/plugin, ./src/services/auth, ./src")]
#[derive(OpenApi)]
#[openapi(
  components(
    schemas(
      cs2kz::SteamID,
      cs2kz::Mode,
      cs2kz::Styles,
      cs2kz::Tier,
      cs2kz::JumpType,
      cs2kz::GlobalStatus,
      cs2kz::RankedStatus,

      crate::util::CourseIdentifier,
      crate::util::MapIdentifier,
      crate::util::PlayerIdentifier,
      crate::util::ServerIdentifier,

      services::steam::WorkshopID,
      services::auth::session::user::Permissions,
      services::players::SessionID,
      services::players::CourseSessionID,
      services::maps::MapID,
      services::maps::CourseID,
      services::maps::FilterID,
      services::servers::ServerID,
      services::records::RecordID,
      services::jumpstats::JumpstatID,
      services::bans::BanID,
      services::bans::UnbanID,
      services::plugin::PluginVersionID,
    )
  ),
  info(
    title = "CS2KZ API",
    description = "\
This is the [OpenAPI] documentation for the CS2KZ API.

The source code is available on [GitHub].

[OpenAPI]: https://www.openapis.org
[RFC 9457]: https://www.rfc-editor.org/rfc/rfc9457.html
[GitHub]: https://github.com/KZGlobalTeam/cs2kz-api",
    license(
      name = "Licensed under the GPL-3.0",
      url = "https://www.gnu.org/licenses/gpl-3.0.html",
    ),
  ),
  external_docs(
    url = "https://docs.cs2kz.org",
    description = "CS2KZ documentation",
  ),
  modifiers(&Security),
)]
pub struct Schema;

impl Schema
{
	/// Returns a [`SwaggerUi`], which can be turned into an [`axum::Router`] to
	/// serve the API's SwaggerUI documentation.
	pub fn swagger_ui() -> SwaggerUi
	{
		SwaggerUi::new("/docs/swagger-ui").url("/docs/openapi.json", Self::openapi())
	}

	/// Generates a JSON representation of the schema.
	///
	/// # Panics
	///
	/// This function will panic if the schema cannot be serialized as JSON.
	pub fn json() -> String
	{
		Self::openapi().to_pretty_json().expect("valid schema")
	}
}
