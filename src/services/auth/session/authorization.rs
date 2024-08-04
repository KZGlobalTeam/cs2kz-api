//! Authorization mechanisms for [`Session`].
//!
//! The main attraction in this module is [`AuthorizeSession`], a trait that
//! describes an authorization method for a session. It is used to dictate -- at
//! compile time, in the type system -- how a session will be authorized at
//! runtime.
//!
//! Authentication mechanisms can have state, but should be cheap to construct
//! and clone.

use std::convert::Infallible;
use std::fmt;
use std::future::Future;

use axum::extract::Request;
use axum::RequestExt;
use sqlx::{MySql, Pool};
use tap::Tap;
use thiserror::Error;

use super::{user, Session};
use crate::http::extract::{Path, PathRejection};
use crate::http::problem_details::{self, IntoProblemDetails, ProblemType};
use crate::services::servers::ServerID;

/// An authorization strategy.
///
/// After a session has been extracted, it can be further processed to ensure
/// the user is authorized to perform the action they're trying to perform. This
/// trait allows you to specify this procedure.
///
/// An error returned from [`authorize_session()`] indicates that the
/// authorization failed.
///
/// The default strategy is [`None`], which does nothing, and therefore always
/// succeeds.
///
/// [`authorize_session()`]: AuthorizeSession::authorize_session
pub trait AuthorizeSession: Clone + Send + Sync + Sized + 'static
{
	/// The error type for this authorization strategy.
	type Error: IntoProblemDetails + Send + Sync + 'static;

	/// Authorize the given session.
	fn authorize_session(
		self,
		session: &Session,
		req: &mut Request,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// The default authorization strategy.
///
/// Any calls to [`None::authorize_session()`] will always succeed and do
/// nothing.
#[derive(Debug, Clone, Copy)]
pub struct None;

impl AuthorizeSession for None
{
	type Error = Infallible;

	#[tracing::instrument(level = "debug", target = "cs2kz_api::auth")]
	async fn authorize_session(
		self,
		session: &Session,
		req: &mut Request,
	) -> Result<(), Self::Error>
	{
		Ok(())
	}
}

/// An authorization strategy that checks if the requesting user has certain
/// permissions.
#[derive(Debug, Clone, Copy)]
pub struct RequiredPermissions(pub user::Permissions);

/// The error that is returned when a user is lacking the required permissions
/// to perform an action.
#[derive(Debug, Error)]
#[error("you do not have the required permissions to perform this action")]
pub struct InsufficientPermissions
{
	/// The permissions that were requried to perform the action.
	required: user::Permissions,

	/// The permissions that the user actually had.
	actual: user::Permissions,
}

impl IntoProblemDetails for InsufficientPermissions
{
	fn problem_type(&self) -> ProblemType
	{
		ProblemType::Unauthorized
	}

	fn add_extension_members(&self, ext: &mut problem_details::ExtensionMembers)
	{
		ext.add("required_permissions", &self.required);
		ext.add("actual_permissions", &self.actual);
	}
}

impl AuthorizeSession for RequiredPermissions
{
	type Error = InsufficientPermissions;

	#[tracing::instrument(level = "debug", target = "cs2kz_api::auth", err(Debug, level = "debug"))]
	async fn authorize_session(
		self,
		session: &Session,
		req: &mut Request,
	) -> Result<(), Self::Error>
	{
		let required = self.0;
		let actual = session.user().permissions();

		if actual.contains(required) {
			Ok(())
		} else {
			Err(InsufficientPermissions { required, actual })
		}
	}
}

/// An authorization strategy that assumes the first path parameter to be a CS2
/// server ID, and checks if the authenticated user owns the server with that
/// ID.
///
/// This will also authorize any requests coming from users with the `SERVERS`
/// permission, as those implicitly have authority over all servers, whether
/// they own them or not.
#[derive(Clone)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct IsServerOwner
{
	database: Pool<MySql>,
}

impl fmt::Debug for IsServerOwner
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("IsServerOwner").finish_non_exhaustive()
	}
}

impl IsServerOwner
{
	/// Creates a new [`IsServerOwner`].
	pub fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}
}

/// Errors for the [`IsServerOwner`] authorization method.
#[derive(Debug, Error)]
pub enum IsServerOwnerError
{
	/// We failed to extract a path parameter from the request.
	#[error(transparent)]
	Path(#[from] PathRejection),

	/// Something went wrong communicating with the database.
	#[error("something went wrong")]
	Database(#[from] sqlx::Error),

	/// The requesting user is not a server owner.
	#[error("you do not own this server")]
	NotServerOwner,
}

impl IntoProblemDetails for IsServerOwnerError
{
	fn problem_type(&self) -> ProblemType
	{
		match self {
			Self::Path(source) => source.problem_type(),
			Self::Database(source) => source.problem_type(),
			Self::NotServerOwner => ProblemType::Unauthorized,
		}
	}
}

impl AuthorizeSession for IsServerOwner
{
	type Error = IsServerOwnerError;

	#[tracing::instrument(
		level = "debug",
		target = "cs2kz_api::auth",
		err(Debug, level = "debug"),
		fields(is_admin = tracing::field::Empty, server.id = tracing::field::Empty),
	)]
	async fn authorize_session(
		self,
		session: &Session,
		req: &mut Request,
	) -> Result<(), Self::Error>
	{
		let is_admin = RequiredPermissions(user::Permissions::SERVERS)
			.authorize_session(session, req)
			.await
			.is_ok();

		let span = tracing::Span::current().tap(|span| {
			span.record("is_admin", is_admin);
		});

		// Admins are always authorized
		if is_admin {
			return Ok(());
		}

		let Path(server_id) = req.extract_parts::<Path<ServerID>>().await?;

		span.record("server.id", format_args!("{server_id}"));

		sqlx::query! {
			r"
			SELECT
			  id
			FROM
			  Servers
			WHERE
			  id = ?
			  AND owner_id = ?
			LIMIT
			  1
			",
			server_id,
			session.user().steam_id(),
		}
		.fetch_optional(&self.database)
		.await?
		.map(|_| ())
		.ok_or(IsServerOwnerError::NotServerOwner)
	}
}
