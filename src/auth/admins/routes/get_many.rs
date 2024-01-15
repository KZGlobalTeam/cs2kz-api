use axum::extract::Query;
use axum::Json;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::admins::Admin;
use crate::auth::{Permission, Permissions};
use crate::extractors::State;
use crate::{responses, Error, Result};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetAdminsParams {
	#[serde(default)]
	pub minimum_permissions: Vec<Permission>,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Auth",
  path = "/auth/admins",
  params(GetAdminsParams),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: State,
	Query(params): Query<GetAdminsParams>,
) -> Result<Json<Vec<Admin>>> {
	let minimum_permissions = params
		.minimum_permissions
		.into_iter()
		.collect::<Permissions>();

	let admins = sqlx::query! {
		r#"
		SELECT
		  p.steam_id `steam_id: SteamID`,
		  p.name,
		  a.permissions
		FROM
		  Admins a
		  JOIN Players p ON p.steam_id = a.steam_id
		WHERE
		  (a.permissions & ?) = ?
		"#,
		minimum_permissions,
		minimum_permissions,
	}
	.fetch_all(state.database())
	.await?
	.into_iter()
	.map(|row| Admin {
		steam_id: row.steam_id,
		name: row.name,
		permissions: Permissions(row.permissions).iter().collect(),
	})
	.collect::<Vec<_>>();

	if admins.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(admins))
}
