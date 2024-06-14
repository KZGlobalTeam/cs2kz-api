//! HTTP handlers for the `/admins` routes.

use axum::Json;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::admins::Admin;
use crate::authorization::Permissions;
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses;
use crate::openapi::responses::PaginationResponse;
use crate::sqlx::query;
use crate::{Error, Result, State};

/// Query parameters for `/admins`.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by permissions.
	#[param(value_type = Vec<String>)]
	#[serde(default)]
	permissions: Permissions,

	/// Maximum number of results to return.
	#[serde(default)]
	limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	offset: Offset,
}

/// Fetch admins (players with permissions).
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/admins",
  tag = "Admins",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<Admin>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: State,
	Query(GetParams {
		permissions,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<PaginationResponse<Admin>>> {
	let mut transaction = state.transaction().await?;

	let admins = sqlx::query_as! {
		Admin,
		r#"
		SELECT SQL_CALC_FOUND_ROWS
		  id `steam_id: SteamID`,
		  name,
		  permissions `permissions: Permissions`
		FROM
		  Players
		WHERE
		  permissions > 0
		  AND (permissions & ?) = ?
		LIMIT
		  ? OFFSET ?
		"#,
		permissions,
		permissions,
		*limit,
		*offset,
	}
	.fetch_all(transaction.as_mut())
	.await?;

	if admins.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: admins,
	}))
}
