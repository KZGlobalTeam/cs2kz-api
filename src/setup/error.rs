//! This module contains the [`Error`] type, an error that can occur during the
//! API's initial setup phase.
//!
//! [`Error`]: enum@Error

use thiserror::Error;

use crate::services::auth;

/// The different errors that can happen in [`server()`].
///
/// [`server()`]: crate::server
#[derive(Debug, Error)]
pub enum Error
{
	/// Something went wrong connecting to the database.
	#[error("failed to setup database")]
	Database(#[from] sqlx::Error),

	/// Something went wrong applying database migrations.
	#[error("failed to run migrations")]
	Migrations(#[from] sqlx::migrate::MigrateError),

	/// Something went wrong initializing the auth service.
	#[error("failed to setup auth service")]
	SetupAuth(#[from] auth::SetupError),
}
