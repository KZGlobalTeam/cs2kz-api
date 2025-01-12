use std::sync::LazyLock;
use std::thread;

use pyo3::Python;
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn for<'py> FnOnce(Python<'py>) + Send>;

static JOBS: LazyLock<mpsc::Sender<Job>> = LazyLock::new(|| {
    let (tx, mut rx) = mpsc::channel::<Job>(16);

    if let Err(error) = thread::Builder::new()
        .name(String::from("pyo3"))
        .spawn(move || {
            Python::with_gil(|py| {
                while let Some(job) = rx.blocking_recv() {
                    job(py);
                }
            })
        })
    {
        panic!("failed to spawn pyo3 thread: {error}");
    }

    tx
});

pub async fn execute<T>(
    span: tracing::Span,
    f: impl for<'py> FnOnce(Python<'py>) -> T + Send + 'static,
) -> T
where
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    let job = Box::new(move |py: Python<'_>| {
        let _ = tx.send(span.in_scope(|| f(py)));
    });

    if JOBS.send(job).await.is_err() {
        panic!("pyo3 thread shut down");
    }

    rx.await.expect("we don't drop the sender")
}
