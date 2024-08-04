//! A very basic service that acts as a healthcheck.
//!
//! This doesn't really need to be a service, but it's the simplest example of
//! one, and can be used as a reference for writing new services.

use std::fmt;

use axum::extract::FromRef;

pub(crate) mod http;

/// A service that simply responds if the API is healthy.
#[derive(Clone, Copy, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct HealthService {}

impl fmt::Debug for HealthService
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("HealthService").finish_non_exhaustive()
	}
}

impl HealthService
{
	/// Create a new [`HealthService`].
	#[tracing::instrument]
	pub fn new() -> Self
	{
		Self {}
	}

	/// Says hello to the world.
	#[tracing::instrument(level = "debug")]
	pub async fn hello(&self) -> &'static str
	{
		"(͡ ͡° ͜ つ ͡͡°)"
	}
}
