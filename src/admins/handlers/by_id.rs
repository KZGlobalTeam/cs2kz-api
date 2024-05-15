//! Handlers for the `/admins/{steam_id}` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::SteamID;
use tracing::trace;

use crate::admins::{Admin, AdminUpdate};
use crate::authorization::{self, Permissions};
use crate::openapi::responses;
use crate::openapi::responses::NoContent;
use crate::{authentication, Error, Result, State};

/// Fetch a specific admin by their SteamID.
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
pub async fn get(state: &State, Path(steam_id): Path<SteamID>) -> Result<Json<Admin>> {
	let admin = sqlx::query! {
		r#"
		SELECT
		  id `id: SteamID`,
		  name,
		  permissions `permissions: Permissions`
		FROM
		  Players
		WHERE
		  id = ?
		"#,
		steam_id,
	}
	.fetch_optional(&state.database)
	.await?
	.map(|row| Admin {
		name: row.name,
		steam_id: row.id,
		permissions: row.permissions,
	})
	.ok_or_else(|| Error::no_content())?;

	Ok(Json(admin))
}

/// Create / update an admin's permissions.
///
/// This will completely replace their previous set of permissions!
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
	state: &State,
	session: authentication::Session<authorization::HasPermissions<{ Permissions::ADMIN.value() }>>,
	Path(steam_id): Path<SteamID>,
	Json(AdminUpdate { permissions }): Json<AdminUpdate>,
) -> Result<NoContent> {
	let mut transaction = state.transaction().await?;

	let query_result = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  permissions = ?
		WHERE
		  id = ?
		"#,
		permissions,
		steam_id,
	}
	.execute(transaction.as_mut())
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("SteamID"));
	}

	transaction.commit().await?;

	trace!(%steam_id, ?permissions, "updated admin");

	Ok(NoContent)
}
