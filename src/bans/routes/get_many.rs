use axum::extract::Query;
use axum::Json;
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use sqlx::QueryBuilder;
use utoipa::IntoParams;

use crate::bans::{queries, Ban};
use crate::database::ToID;
use crate::extract::State;
use crate::params::{Limit, Offset};
use crate::query::{self, Filter};
use crate::{responses, Error, Result};

/// Query Parameters for fetching [`Ban`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetBansParams<'a> {
	/// Filter by player.
	///
	/// This can be either a SteamID or name.
	pub player: Option<PlayerIdentifier<'a>>,

	/// Filter by ban reason.
	pub reason: Option<String>,

	/// Filter by server.
	///
	/// This can either be an ID or name.
	pub server: Option<ServerIdentifier<'a>>,

	/// Filter by admin.
	///
	/// This can be either a SteamID or name.
	pub banned_by: Option<PlayerIdentifier<'a>>,

	/// Filter by bans which have (not) expired yet.
	pub has_expired: Option<bool>,

	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

/// Player Bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Bans",
  path = "/bans",
  params(GetBansParams),
  responses(
    responses::Ok<Ban>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: State,
	Query(params): Query<GetBansParams<'_>>,
) -> Result<Json<Vec<Ban>>> {
	let mut query = QueryBuilder::new(queries::BASE_SELECT);
	let mut filter = Filter::new();

	if let Some(ref player) = params.player {
		let steam_id = player.to_id(state.database()).await?;

		query
			.push(filter)
			.push(" b.player_id = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(ref reason) = params.reason {
		query
			.push(filter)
			.push(" b.reason LIKE ")
			.push_bind(format!("%{reason}%"));

		filter.switch();
	}

	if let Some(ref server) = params.server {
		let server_id = server.to_id(state.database()).await?;

		query
			.push(filter)
			.push(" b.server_id = ")
			.push_bind(server_id);

		filter.switch();
	}

	if let Some(ref player) = params.banned_by {
		let steam_id = player.to_id(state.database()).await?;

		query
			.push(filter)
			.push(" b.banned_by = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(has_expired) = params.has_expired {
		query
			.push(filter)
			.push(" b.expires_on ")
			.push(if has_expired { "<" } else { ">" })
			.push(" CURRENT_TIMESTAMP() ");

		filter.switch();
	}

	query.push(" ORDER BY b.id DESC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let bans = query
		.build_query_as::<Ban>()
		.fetch_all(state.database())
		.await?;

	if bans.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(bans))
}
