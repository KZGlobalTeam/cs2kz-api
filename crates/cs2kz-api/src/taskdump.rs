use std::time::Duration;

use axum::response::{IntoResponse, Response};
use axum::routing::{self, Router};
use tokio::runtime::Handle as RuntimeHandle;
use tokio_util::time::FutureExt;

use crate::extract::Json;

pub fn router<S>() -> Router<S> {
    Router::new().route("/", routing::get(get)).with_state(())
}

#[derive(Debug, Display, Error, From)]
#[display("failed to capture taskdump within reasonable timeout")]
struct Timeout(tokio::time::error::Elapsed);

/// Captures a [taskdump] and returns an array of tracebacks.
///
/// [taskdump]: tokio::runtime::Handle::dump
#[tracing::instrument(err)]
async fn get() -> Result<Json<Vec<String>>, Timeout> {
    info!("capturing taskdump");

    let dump = RuntimeHandle::current()
        .dump()
        .timeout(Duration::from_secs(5))
        .await?;

    let traces = dump
        .tasks()
        .iter()
        .map(|task| task.trace().to_string())
        .collect();

    Ok(Json(traces))
}

impl IntoResponse for Timeout {
    fn into_response(self) -> Response {
        (http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
