use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

use color_eyre::eyre::{eyre, WrapErr};
use console_subscriber::{ConsoleLayer, ServerAddr};
use tracing_subscriber::registry::LookupSpan;

/// Creates a tracing layer that will send telemetry to [tokio-console].
///
/// [tokio-console]: https://github.com/tokio-rs/console
pub fn layer<S>() -> color_eyre::Result<impl tracing_subscriber::Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	let mut layer = ConsoleLayer::builder();

	if let Ok(value) = env::var("TOKIO_LOG") {
		let addr = if let Ok(addr) = value.parse::<SocketAddr>() {
			ServerAddr::Tcp(addr)
		} else if let Ok(path) = value.parse::<PathBuf>() {
			ServerAddr::Unix(path)
		} else {
			return Err(eyre!("invalid `TOKIO_LOG` value"))
				.context("must be tcp socket address or UDS path");
		};

		layer = layer.server_addr(addr);
	}

	Ok(layer.spawn())
}
