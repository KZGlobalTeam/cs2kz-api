//! Handlers for the `/admins` route.

use axum::Json;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use futures::TryStreamExt;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::admins::Admin;
use crate::auth::RoleFlags;
use crate::parameters::{Limit, Offset};
use crate::responses::PaginationResponse;
use crate::sqlx::query;
use crate::{responses, Error, Result, State};

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
    responses::Ok<PaginationResponse<Admin>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &'static State,
	Query(GetParams { roles, limit, offset }): Query<GetParams>,
) -> Result<Json<PaginationResponse<Admin>>> {
	let mut transaction = state.transaction().await?;

	let admins = sqlx::query! {
		r#"
		SELECT SQL_CALC_FOUND_ROWS
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
	.fetch(transaction.as_mut())
	.map_ok(|row| Admin { name: row.name, steam_id: row.id, roles: row.role_flags })
	.try_collect::<Vec<_>>()
	.await?;

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	if admins.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(PaginationResponse { total, results: admins }))
}
