//! Everything related to authorization.
//!
//! [Sessions][session] are parameterized by an authorization method `A`. This module defines the
//! [`AuthorizeSession`] trait which that parameter is bound by, as well as implementations of that
//! trait. This `A` parameter can be used to control the authorization method for any given
//! [session] at the type system level.
//!
//! [session]: crate::authentication::Session

use std::future::Future;

use axum::http::request;
use sqlx::{MySql, Transaction};

use crate::{authentication, Result};

mod permissions;
pub use permissions::Permissions;

mod none;
pub use none::None;

mod has_permissions;
pub use has_permissions::HasPermissions;

mod is_server_admin_or_owner;
pub use is_server_admin_or_owner::IsServerAdminOrOwner;

/// A trait used for authorizing a [session].
///
/// See [module level docs] for more details.
///
/// [session]: crate::authentication::Session
/// [module level docs]: crate::authorization
pub trait AuthorizeSession: Send + Sync + 'static {
	/// Authorize the session of the given `user`.
	fn authorize_session(
		user: &authentication::User,
		req: &mut request::Parts,
		transaction: &mut Transaction<'_, MySql>,
	) -> impl Future<Output = Result<()>> + Send;
}
