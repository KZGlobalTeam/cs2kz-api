/// API Healthcheck.
///
/// If this endpoint responds, it means the API is up and running.
#[utoipa::path(get, tag = "Health", context_path = "/api", path = "/", responses(
	(status = 200, content_type = "text/plain", description = "The API is healthy.")
))]
pub async fn health() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
