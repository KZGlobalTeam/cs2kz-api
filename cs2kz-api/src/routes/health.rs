#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Health", path = "/", responses(
	(status = 200, content_type = "text/plain", description = "The API is healthy.")
))]
pub async fn health() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
