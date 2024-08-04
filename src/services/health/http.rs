//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::HealthService;

impl From<HealthService> for Router
{
	fn from(svc: HealthService) -> Self
	{
		Router::new().route("/", routing::get(get)).with_state(svc)
	}
}

/// (͡ ͡° ͜ つ ͡͡°)
#[tracing::instrument]
#[utoipa::path(get, path = "/", tag = "Health", responses(
  (status = OK, description = "The API is healthy.", body = str),
))]
async fn get(State(svc): State<HealthService>) -> &'static str
{
	svc.hello().await
}
