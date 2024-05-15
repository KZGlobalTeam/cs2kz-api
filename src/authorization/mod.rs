//! Everything related to authorization

use std::future::Future;

use axum::http::request;
use sqlx::{MySql, Transaction};

use crate::authentication;

mod permissions;

#[doc(inline)]
pub use permissions::Permissions;

mod none;

#[doc(inline)]
pub use none::None;

mod has_permissions;

#[doc(inline)]
pub use has_permissions::HasPermissions;

mod is_server_admin_or_owner;

#[doc(inline)]
pub use is_server_admin_or_owner::IsServerAdminOrOwner;

/// Used for deciding an authorization strategy when doing [session authentication].
///
/// [session authentication]: crate::authentication::session
pub trait AuthorizeSession: Send + Sync + 'static {
	/// Authorize a session for the given `user`.
	fn authorize_session(
		user: &authentication::User,
		req: &mut request::Parts,
		transaction: &mut Transaction<'static, MySql>,
	) -> impl Future<Output = crate::Result<()>> + Send;
}
