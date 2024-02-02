use axum::Json;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use itertools::Itertools;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::admins::Admin;
use crate::auth::{Role, RoleFlags};
use crate::params::{Limit, Offset};
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching KZ admins.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetAdminsParams {
	/// Only include admins with these roles.
	pub roles: Vec<Role>,

	/// Maximum amount of results.
	#[param(value_type = Option<u64>, maximum = 1000)]
	pub limit: Limit,

	/// Offset used for pagination.
	#[param(value_type = Option<i64>)]
	pub offset: Offset,
}

/// Fetch users with elevated permissions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Admins",
  path = "/admins",
  params(GetAdminsParams),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetAdminsParams>,
) -> Result<Json<Vec<Admin>>> {
	let role_flags = params.roles.into_iter().collect::<RoleFlags>();

	let admins = sqlx::query! {
		r#"
		SELECT
		  name,
		  steam_id `steam_id: SteamID`,
		  role_flags `role_flags: RoleFlags`
		FROM
		  Players
		WHERE
		  role_flags > 0
		  AND (role_flags & ?) = ?
		LIMIT
		  ? OFFSET ?
		"#,
		role_flags,
		role_flags,
		params.limit,
		params.offset,
	}
	.fetch_all(&state.database)
	.await?
	.into_iter()
	.map(|row| Admin {
		name: row.name,
		steam_id: row.steam_id,
		roles: row.role_flags.iter().collect(),
	})
	.collect_vec();

	if admins.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(admins))
}
