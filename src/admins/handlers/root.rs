//! Handlers for the `/admins` route.

use axum::Json;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use itertools::Itertools;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::admins::Admin;
use crate::auth::RoleFlags;
use crate::parameters::{Limit, Offset};
use crate::{responses, AppState, Error, Result};

/// Query parameters for `GET /admins`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by roles.
	#[param(value_type = Vec<String>)]
	#[serde(default)]
	roles: RoleFlags,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/admins",
  tag = "Admins",
  params(GetParams),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: AppState,
	Query(GetParams { roles, limit, offset }): Query<GetParams>,
) -> Result<Json<Vec<Admin>>> {
	let admins = sqlx::query! {
		r#"
		SELECT
		  id `id: SteamID`,
		  name,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Players
		WHERE
		  role_flags > 0
		  AND (role_flags & ?) = ?
		LIMIT
		  ? OFFSET ?
		"#,
		roles,
		roles,
		limit.0,
		offset.0,
	}
	.fetch_all(&state.database)
	.await?
	.into_iter()
	.map(|row| Admin { name: row.name, steam_id: row.id, roles: row.role_flags })
	.collect_vec();

	if admins.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(admins))
}
