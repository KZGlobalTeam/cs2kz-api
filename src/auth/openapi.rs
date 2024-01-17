use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::OpenApi;
use utoipa::Modify;

use crate::extractors::SessionToken;

pub struct Security;

impl Modify for Security {
	fn modify(&self, openapi: &mut OpenApi) {
		let components = openapi
			.components
			.as_mut()
			.expect("OpenAPI Spec has components");

		let session_auth =
			SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SessionToken::COOKIE_NAME)));

		components.add_security_scheme("Steam Session", session_auth);

		let server_jwt_auth = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));

		components.add_security_scheme("CS2 Server JWT", server_jwt_auth);
	}
}
