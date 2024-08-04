//! This module contains OS signal handlers.
//!
//! This is currently only used for graceful shutdown on SIGINT.

use tokio::signal;

/// The future returned by this function will resolve when the program receives
/// a `SIGINT` signal from the OS.
#[tracing::instrument]
pub async fn sigint()
{
	match signal::ctrl_c().await {
		Ok(()) => tracing::warn!("received SIGINT, shutting down"),
		Err(error) => tracing::error!(%error, "failed to receive SIGINT"),
	}
}
