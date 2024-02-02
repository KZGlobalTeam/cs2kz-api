use axum::http::Method;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;

pub fn router() -> Router {
	Router::new().route(
		"/",
		get(status).route_layer(CorsLayer::permissive().allow_methods(Method::GET)),
	)
}

/// The API is up and running!
#[tracing::instrument]
#[utoipa::path(
  get,
  tag = "Status",
  path = "/",
  responses((status = OK, description = "(͡ ͡° ͜ つ ͡͡°)")),
)]
pub async fn status() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
