use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::query::FilteredQuery;
use crate::sessions::{queries, Session};
use crate::{query, responses, AppState, Error, Result};

#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetSessionsParams<'a> {
	/// Filter by a specific player.
	pub player: Option<PlayerIdentifier<'a>>,

	/// Filter by a specific server.
	pub server: Option<ServerIdentifier<'a>>,

	/// Only include sessions after this timestamp.
	pub after: Option<DateTime<Utc>>,

	/// Only include sessions before this timestamp.
	pub before: Option<DateTime<Utc>>,

	/// Maximum amount of results.
	pub limit: Limit,

	/// Offset used for pagination.
	pub offset: Offset,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  tag = "Sessions",
  path = "/sessions",
  params(GetSessionsParams<'_>),
  responses(
    responses::Ok<Session>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get_many(
	state: AppState,
	Query(params): Query<GetSessionsParams<'_>>,
) -> Result<Json<Vec<Session>>> {
	let mut query = FilteredQuery::new(queries::BASE_SELECT);

	if let Some(player) = params.player {
		let steam_id = player.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" p.steam_id = ").push_bind(steam_id);
		});
	}

	if let Some(server) = params.server {
		let server_id = server.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" sv.id = ").push_bind(server_id);
		});
	}

	if let Some(after) = params.after {
		query.filter(|query| {
			query.push(" s.created_on > ").push_bind(after);
		});
	}

	if let Some(before) = params.before {
		query.filter(|query| {
			query.push(" s.created_on < ").push_bind(before);
		});
	}

	query.push(" ORDER BY s.id DESC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let sessions = query
		.build_query_as::<Session>()
		.fetch_all(&state.database)
		.await?;

	if sessions.is_empty() {
		return Err(Error::no_data());
	}

	Ok(Json(sessions))
}
