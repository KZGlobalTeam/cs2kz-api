#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Health", context_path = "/api/v1", path = "/health", responses(
	(status = 200, content_type = "text/plain", description = "The API is healthy.")
))]
pub async fn health() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
