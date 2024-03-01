use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, Layer};

/// Provides a tracing layer for tokio-console.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	console_subscriber::spawn().with_filter(EnvFilter::new("tokio=trace"))
}
