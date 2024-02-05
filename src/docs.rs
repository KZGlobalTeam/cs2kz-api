use std::sync::OnceLock;

use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::OpenApi;
use utoipa::OpenApi as _;

use crate::API;

static SPEC: OnceLock<OpenApi> = OnceLock::new();
static SWAGGER_UI: OnceLock<String> = OnceLock::new();

pub fn router() -> Router {
	let spec = SPEC.get_or_init(API::openapi);
	let swagger_ui = SWAGGER_UI
		.get_or_init(|| axum_swagger_ui::swagger_ui("/docs/open-api.json"))
		.as_str();

	Router::new()
		.route("/open-api.json", get(move || async move { Json(spec) }))
		.route("/swagger-ui", get(move || async move { Html(swagger_ui) }))
}
