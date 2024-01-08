//! Middleware for requests coming from websites such as `https://cs2.kz`

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use cs2kz::SteamID;
use tracing::trace;

use crate::extractors::SessionToken;
use crate::permissions::Permissions;
use crate::{Error, Result, State};

/// The name of the authentication cookie for logged in users.
pub static COOKIE: &str = "kz-auth";

/// A logged in admin.
#[derive(Debug, Clone, Copy)]
pub struct Admin {
	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's permissions for this request.
	pub permissions: Permissions,
}

/// Verifies a request coming from a website such as `https://forum.cs2.kz/`.
#[tracing::instrument(skip_all, ret, err(Debug))]
pub async fn verify_web_user<const MIN_PERMS: u64>(
	state: State,
	token: SessionToken,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	verify::<MIN_PERMS>(state, token, &mut request).await?;

	Ok(next.run(request).await)
}

pub(super) async fn verify<const MIN_PERMS: u64>(
	state: State,
	SessionToken(token): SessionToken,
	request: &mut Request,
) -> Result<()> {
	let subdomain = request
		.uri()
		.host()
		.and_then(|domain| domain.split_once('.'))
		.map(|(subdomain, _)| subdomain)
		.ok_or_else(|| {
			trace!("invalid domain");
			Error::Unauthorized
		})?;

	let user = sqlx::query! {
		r#"
		SELECT
		  *
		FROM
		  Admins
		WHERE
		  steam_id = (
		    SELECT
		      steam_id
		    FROM
		      WebSessions
		    WHERE
		      token = ?
		      AND subdomain = ?
		      AND expires_on < CURRENT_TIMESTAMP()
		  )
		"#,
		token,
		subdomain,
	}
	.fetch_optional(state.database())
	.await?
	.ok_or_else(|| {
		trace!("missing session");
		Error::Unauthorized
	})?;

	if !Permissions(user.permissions).contains(Permissions(MIN_PERMS)) {
		trace!("insufficient permissions");
		return Err(Error::Unauthorized);
	}

	let admin = Admin {
		steam_id: user
			.steam_id
			.try_into()
			.expect("invalid SteamID in database"),
		permissions: Permissions(user.permissions),
	};

	request.extensions_mut().insert(admin);

	Ok(())
}
