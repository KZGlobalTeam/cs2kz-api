//! Tracing layer for [tokio-console].
//!
//! [tokio-console]: https://github.com/tokio-rs/console

use console_subscriber::{ConsoleLayer, ServerAddr};
use cs2kz_api::runtime::config::{TokioConsoleServerAddr, TracingConsoleConfig};
use tap::Pipe;
use tracing_subscriber::registry::LookupSpan;

/// Creates a tracing layer that will send telemetry to [tokio-console].
///
/// [tokio-console]: https://github.com/tokio-rs/console
pub fn layer<S>(
	config: TracingConsoleConfig,
) -> color_eyre::Result<impl tracing_subscriber::Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	let layer = ConsoleLayer::builder().pipe(|layer| match config.server_addr {
		None => layer,
		Some(TokioConsoleServerAddr::Tcp(addr)) => layer.server_addr(ServerAddr::Tcp(addr)),
		Some(TokioConsoleServerAddr::Unix(path)) => layer.server_addr(ServerAddr::Unix(path)),
	});

	Ok(layer.spawn())
}
