use axum::http::Method;
use axum::routing::get;
use axum::Router;

use crate::cors;

pub fn router() -> Router {
	Router::new()
		.route("/", get(status))
		.route_layer(cors::permissive(Method::GET))
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
