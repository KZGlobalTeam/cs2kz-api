use axum::extract::Query;
use axum::Json;
use cs2kz::{PlayerIdentifier, ServerIdentifier};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::auth::sessions::Admin;
use crate::auth::{Role, Session};
use crate::bans::{queries, Ban, BanReason};
use crate::database::ToID;
use crate::params::{Limit, Offset};
use crate::query::{self, FilteredQuery};
use crate::{responses, AppState, Error, Result};

/// Query Parameters for fetching [`Ban`]s.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
pub struct GetBansParams<'a> {
	/// Filter by player.
	///
	/// This can be either a SteamID or name.
	pub player: Option<PlayerIdentifier<'a>>,

	/// Filter by ban reason.
	pub reason: Option<BanReason>,

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

/// Fetch bans.
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
	state: AppState,
	session: Option<Session<Admin<{ Role::Bans as u32 }>>>,
	Query(params): Query<GetBansParams<'_>>,
) -> Result<Json<Vec<Ban>>> {
	let mut query = FilteredQuery::new(queries::BASE_SELECT);

	if let Some(ref player) = params.player {
		let steam_id = player.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" b.player_id = ").push_bind(steam_id);
		});
	}

	if let Some(ref reason) = params.reason {
		query.filter(|query| {
			query.push(" b.reason LIKE ").push_bind(reason);
		});
	}

	if let Some(ref server) = params.server {
		let server_id = server.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" b.server_id = ").push_bind(server_id);
		});
	}

	if let Some(ref player) = params.banned_by {
		let steam_id = player.to_id(&state.database).await?;

		query.filter(|query| {
			query.push(" b.banned_by = ").push_bind(steam_id);
		});
	}

	if let Some(has_expired) = params.has_expired {
		query.filter(|query| {
			query.push(if has_expired {
				" b.expires_on < CURRENT_TIMESTAMP() "
			} else {
				" (b.expires_on > CURRENT_TIMESTAMP() OR b.expires_on IS NULL) "
			});
		});
	}

	query.push(" ORDER BY b.id DESC ");
	query::push_limit(params.limit, params.offset, &mut query);

	let mut bans = query
		.build_query_as::<Ban>()
		.fetch_all(&state.database)
		.await?;

	if bans.is_empty() {
		return Err(Error::no_data());
	}

	if session.is_none() {
		for ban in &mut bans {
			ban.player.ip_address = None;
		}
	}

	Ok(Json(bans))
}
