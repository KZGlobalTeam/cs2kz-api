//! Middleware for requests coming from websites such as `https://cs2.kz`

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use cs2kz::SteamID;
use time::OffsetDateTime;
use tracing::trace;

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
	cookies: CookieJar,
	mut request: Request,
	next: Next,
) -> Result<Response> {
	verify::<MIN_PERMS>(state, cookies, &mut request).await?;

	Ok(next.run(request).await)
}

pub(super) async fn verify<const MIN_PERMS: u64>(
	state: State,
	cookies: CookieJar,
	request: &mut Request,
) -> Result<()> {
	let cookie = cookies.get(COOKIE).ok_or_else(|| {
		trace!("missing cookie");
		Error::Unauthorized
	})?;

	let is_expired = cookie
		.expires_datetime()
		.map(OffsetDateTime::unix_timestamp)
		.is_some_and(|expires_at| expires_at < Utc::now().timestamp());

	if is_expired {
		trace!("expired cookie");
		return Err(Error::Unauthorized);
	}

	let token = cookie.value().parse::<u64>().map_err(|_| {
		trace!("invalid cookie");
		Error::Unauthorized
	})?;

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
