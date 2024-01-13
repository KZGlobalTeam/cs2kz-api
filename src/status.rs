/// The API is up and running!
#[tracing::instrument]
#[utoipa::path(
  get,
  tag = "Status",
  path = "/",
  responses((status = OK, description = "(͡ ͡° ͜ つ ͡͡°)")),
)]
pub async fn hello_world() -> &'static str {
	"(͡ ͡° ͜ つ ͡͡°)"
}
