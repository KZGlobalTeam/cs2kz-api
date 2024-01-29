use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;
use sqlx::QueryBuilder;

use crate::auth::{Role, RoleFlags};
use crate::{responses, AppState, Error, Result};

/// Get a specific player by SteamID or name.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/{player}/roles",
  params(PlayerIdentifier<'_>),
  responses(
    responses::Ok<Role>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_roles(
	state: AppState,
	Path(player): Path<PlayerIdentifier<'_>>,
) -> Result<Json<Vec<Role>>> {
	let mut query = QueryBuilder::new("SELECT role_flags FROM Players WHERE");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push(" steam_id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push(" name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let role_flags = query
		.build_query_scalar::<u32>()
		.fetch_optional(state.database())
		.await?
		.map(RoleFlags)
		.ok_or(Error::NoContent)?;

	Ok(Json(role_flags.into_iter().collect()))
}
