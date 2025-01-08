use axum::extract::State;
use axum::routing::{self, Router};
use tokio::runtime::{Handle as RuntimeHandle, RuntimeMetrics};
use tower_http::metrics::in_flight_requests::InFlightRequestsCounter;

use crate::extract::Json;

#[derive(Clone)]
struct MetricsState {
    runtime: RuntimeMetrics,
    request_counter: InFlightRequestsCounter,
}

pub fn router<S>(request_counter: InFlightRequestsCounter) -> Router<S> {
    Router::new()
        .route("/", routing::get(get))
        .with_state(MetricsState {
            runtime: RuntimeHandle::current().metrics(),
            request_counter,
        })
}

#[derive(Debug, serde::Serialize)]
struct Metrics {
    worker_threads: usize,
    blocking_threads: usize,
    idle_blocking_threads: usize,
    active_tasks: usize,
    spawned_tasks: u64,
    in_flight_requests: usize,
}

#[tracing::instrument(skip(metrics), ret)]
async fn get(State(metrics): State<MetricsState>) -> Json<Metrics> {
    Json(Metrics {
        worker_threads: metrics.runtime.num_workers(),
        blocking_threads: metrics.runtime.num_blocking_threads(),
        idle_blocking_threads: metrics.runtime.num_idle_blocking_threads(),
        active_tasks: metrics.runtime.num_alive_tasks(),
        spawned_tasks: metrics.runtime.spawned_tasks_count(),
        in_flight_requests: metrics.request_counter.get(),
    })
}
