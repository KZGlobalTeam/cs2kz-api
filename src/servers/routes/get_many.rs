use axum::extract::Query;
use axum::Json;
use cs2kz::PlayerIdentifier;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::query::{self, FilteredQuery};
use crate::servers::{queries, Server};
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching [`Server`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetServersParams<'a> {
	/// Filter by name.
	pub name: Option<String>,

	/// Filter by server owner.
	///
	/// This can be either a SteamID or name.
	pub owner: Option<PlayerIdentifier<'a>>,

	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

/// Fetch registered CS2 servers.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Servers",
  path = "/servers",
  params(GetServersParams),
  responses(
    responses::Ok<Server>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetServersParams<'_>>,
) -> Result<Json<Vec<Server>>> {
	let mut query = FilteredQuery::new(queries::BASE_SELECT);

	if let Some(ref name) = params.name {
		query.filter(|query| {
			query.push(" s.name LIKE ").push_bind(format!("%{name}%"));
		});
	}

	if let Some(ref player) = params.owner {
		let steam_id = player.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" p.steam_id = ").push_bind(steam_id);
		});
	}

	query.push(" ORDER BY s.id ASC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let servers = query
		.build_query_as::<Server>()
		.fetch_all(&state.database)
		.await?;

	if servers.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(servers))
}
