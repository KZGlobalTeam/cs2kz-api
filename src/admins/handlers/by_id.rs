//! Handlers for the `/admins/{steam_id}` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;
use tracing::trace;

use crate::admins::{Admin, AdminUpdate};
use crate::auth::{self, RoleFlags};
use crate::responses::NoContent;
use crate::{responses, Error, Result, State};

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/admins/{steam_id}",
  tag = "Admins",
  params(SteamID),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: &'static State, Path(steam_id): Path<SteamID>) -> Result<Json<Admin>> {
	let admin = sqlx::query! {
		r#"
		SELECT
		  id `id: SteamID`,
		  name,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Players
		WHERE
		  id = ?
		"#,
		steam_id,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| Admin { name: row.name, steam_id: row.id, roles: row.role_flags })
	.ok_or_else(|| Error::no_content())?;

	Ok(Json(admin))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  put,
  path = "/admins/{steam_id}",
  tag = "Admins",
  security(("Browser Session" = ["admins"])),
  params(SteamID),
  request_body = AdminUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn put(
	state: &'static State,
	session: auth::Session<auth::HasRoles<{ RoleFlags::ADMIN.as_u32() }>>,
	Path(steam_id): Path<SteamID>,
	Json(AdminUpdate { roles }): Json<AdminUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  role_flags = ?
		WHERE
		  id = ?
		"#,
		roles,
		steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID"));
	}

	transaction.commit().await?;

	trace!(%steam_id, ?roles, "updated admin");

	Ok(NoContent)
}
