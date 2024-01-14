use axum::extract::Query;
use axum::Json;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::admins::Admin;
use crate::auth::Permissions;
use crate::extractors::State;
use crate::{responses, Error, Result};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetAdminsParams {
	#[serde(default)]
	pub minimum_permissions: Permissions,
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
	let admins = sqlx::query_as! {
		Admin,
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
		params.minimum_permissions,
		params.minimum_permissions,
	}
	.fetch_all(state.database())
	.await?;

	if admins.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(admins))
}
