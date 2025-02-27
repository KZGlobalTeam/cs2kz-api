use std::sync::LazyLock;
use std::thread;

use pyo3::types::PyAnyMethods as _;
use pyo3::{PyAny, PyResult, Python};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn for<'a, 'py> FnOnce(PyCtx<'a, 'py>) + Send>;

static JOBS: LazyLock<mpsc::Sender<Job>> = LazyLock::new(|| {
    fn import_norminvgauss(py: Python<'_>) -> PyResult<pyo3::Bound<'_, PyAny>> {
        py.import("scipy.stats")?.getattr("norminvgauss")
    }

    fn import_quad(py: Python<'_>) -> PyResult<pyo3::Bound<'_, PyAny>> {
        py.import("scipy")?.getattr("integrate")?.getattr("quad")
    }

    let (tx, mut rx) = mpsc::channel::<Job>(16);

    if let Err(error) = thread::Builder::new()
        .name(String::from("pyo3"))
        .spawn(move || {
            Python::with_gil(|py| {
                let norminvgauss = import_norminvgauss(py).expect("failed to import norminvgauss");

                let fit = norminvgauss
                    .getattr("fit")
                    .expect("failed to import norminvgauss fit function");

                let pdf = norminvgauss
                    .getattr("_pdf")
                    .expect("failed to import norminvgauss pdf function");

                let quad = import_quad(py).expect("failed to import quad integrate");

                while let Some(job) = rx.blocking_recv() {
                    job(PyCtx {
                        py,
                        norminvgauss: &norminvgauss,
                        fit: &fit,
                        pdf: &pdf,
                        quad: &quad,
                    });
                }

                warn!("pyo3 thread exiting");
            })
        })
    {
        panic!("failed to spawn pyo3 thread: {error}");
    }

    tx
});

#[derive(Clone, Copy)]
pub struct PyCtx<'a, 'py> {
    pub py: Python<'py>,
    pub norminvgauss: &'a pyo3::Bound<'py, PyAny>,
    pub fit: &'a pyo3::Bound<'py, PyAny>,
    pub pdf: &'a pyo3::Bound<'py, PyAny>,
    pub quad: &'a pyo3::Bound<'py, PyAny>,
}

pub async fn execute<T>(
    span: tracing::Span,
    f: impl for<'a, 'py> FnOnce(PyCtx<'a, 'py>) -> T + Send + 'static,
) -> T
where
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    let job = Box::new(move |cx: PyCtx<'_, '_>| {
        let _ = tx.send(span.in_scope(|| f(cx)));
    });

    if JOBS.send(job).await.is_err() {
        panic!("pyo3 thread shut down");
    }

    rx.await.expect("we don't drop the sender")
}
