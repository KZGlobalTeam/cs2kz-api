//! Security modifiers for the OpenAPI spec.

use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::OpenApi;

/// Security modifier for the OpenAPI spec.
pub struct Security;

impl utoipa::Modify for Security
{
	fn modify(&self, openapi: &mut OpenApi)
	{
		let sessions = SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(
			crate::services::auth::session::COOKIE_NAME,
		)));

		let cs_server_jwt = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));
		let api_key = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));
		let components = openapi.components.get_or_insert_with(Default::default);

		components.add_security_schemes_from_iter([
			("Browser Session", sessions),
			("CS2 Server", cs_server_jwt),
			("API Key", api_key),
		])
	}
}
