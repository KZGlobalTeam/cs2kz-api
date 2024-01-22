use axum::extract::Query;
use axum::Json;
use cs2kz::SteamID;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::RoleFlags;
use crate::extract::State;
use crate::params::{Limit, Offset};
use crate::players::routes::get_many::GetPlayersParams;
use crate::players::Admin;
use crate::{responses, Error, Result};

/// Query Parameters for fetching [`Admin`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetAdminsParams {
	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

/// Players who have joined officially approved KZ servers.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/admins",
  params(GetPlayersParams),
  responses(
    responses::Ok<Admin>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_admins(
	state: State,
	Query(params): Query<GetAdminsParams>,
) -> Result<Json<Vec<Admin>>> {
	let admins = sqlx::query! {
		r#"
		SELECT
		  steam_id `steam_id: SteamID`,
		  name,
		  role_flags
		FROM
		  Players
		WHERE
		  role_flags != 0
		LIMIT
		  ? OFFSET ?
		"#,
		params.limit,
		params.offset,
	}
	.fetch_all(state.database())
	.await?
	.into_iter()
	.map(|row| Admin {
		steam_id: row.steam_id,
		name: row.name,
		roles: Vec::from_iter(RoleFlags(row.role_flags)),
	})
	.collect::<Vec<_>>();

	if admins.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(admins))
}
