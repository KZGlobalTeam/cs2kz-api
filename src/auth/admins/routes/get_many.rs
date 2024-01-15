use axum::Json;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::admins::Admin;
use crate::auth::{Role, RoleFlags};
use crate::extractors::State;
use crate::{responses, Error, Result};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetAdminsParams {
	#[serde(default)]
	pub required_roles: Vec<Role>,
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
	let required_roles = params.required_roles.into_iter().collect::<RoleFlags>();

	let admins = sqlx::query! {
		r#"
		SELECT
		  p.steam_id `steam_id: SteamID`,
		  p.name,
		  a.role_flags
		FROM
		  Admins a
		  JOIN Players p ON p.steam_id = a.steam_id
		WHERE
		  (a.role_flags & ?) = ?
		"#,
		required_roles,
		required_roles,
	}
	.fetch_all(state.database())
	.await?
	.into_iter()
	.map(|row| Admin {
		steam_id: row.steam_id,
		name: row.name,
		roles: RoleFlags(row.role_flags).iter().collect(),
	})
	.collect::<Vec<_>>();

	if admins.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(admins))
}
