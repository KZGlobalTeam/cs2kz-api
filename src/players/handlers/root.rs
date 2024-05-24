//! Handlers for the `/players` route.

use axum::extract::Query;
use axum::Json;
use futures::TryStreamExt;
use serde::Deserialize;
use sqlx::QueryBuilder;
use tracing::info;
use utoipa::IntoParams;

use crate::authentication::Jwt;
use crate::authorization::Permissions;
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses::{self, Created, PaginationResponse};
use crate::players::{queries, FullPlayer, NewPlayer};
use crate::sqlx::{query, QueryBuilderExt, SqlErrorExt};
use crate::{authentication, authorization, Error, Result, State};

/// Query parameters for `GET /players`.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct GetParams {
	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Fetch players.
///
/// If you send a cookie that shows you're "logged in", and you happen to have permissions for
/// managing bans, the response will include IP addresses.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/players",
  tag = "Players",
  params(GetParams),
  responses(
    responses::Ok<PaginationResponse<FullPlayer>>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &State,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	Query(GetParams { limit, offset }): Query<GetParams>,
) -> Result<Json<PaginationResponse<FullPlayer>>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push_limits(limit, offset);

	let mut transaction = state.transaction().await?;

	let players = query
		.build_query_as::<FullPlayer>()
		.fetch(transaction.as_mut())
		.map_ok(|player| FullPlayer {
			// Only include IP address information if the requesting user has
			// permission to view them.
			ip_address: session.as_ref().and(player.ip_address),
			..player
		})
		.try_collect::<Vec<_>>()
		.await?;

	if players.is_empty() {
		return Err(Error::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(PaginationResponse {
		total,
		results: players,
	}))
}

/// Register a new player.
///
/// This endpoint will be hit by CS2 servers whenever an unknown player joins.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/players",
  tag = "Players",
  security(("CS2 Server" = [])),
  request_body = NewPlayer,
  responses(
    responses::Created,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: &State,
	Jwt {
		payload: server, ..
	}: Jwt<authentication::Server>,
	Json(NewPlayer {
		name,
		steam_id,
		ip_address,
	}): Json<NewPlayer>,
) -> Result<Created> {
	sqlx::query! {
		r#"
		INSERT INTO
		  Players (id, name, ip_address)
		VALUES
		  (?, ?, ?)
		"#,
		steam_id,
		name,
		ip_address,
	}
	.execute(&state.database)
	.await
	.map_err(|err| {
		if err.is_duplicate_entry() {
			Error::already_exists("player").with_source(err)
		} else {
			Error::from(err)
		}
	})?;

	info!(target: "audit_log", %name, %steam_id, "registered new player");

	Ok(Created(()))
}

#[cfg(test)]
mod tests {
	use std::net::Ipv6Addr;
	use std::time::Duration;

	use cs2kz::SteamID;
	use tokio::time::sleep;

	use crate::openapi::responses::PaginationResponse;
	use crate::players::{FullPlayer, NewPlayer};

	#[crate::integration_test]
	async fn fetch_players(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/players"))
			.query(&[("limit", "7")])
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let response = response.json::<PaginationResponse<FullPlayer>>().await?;

		assert!(response.results.len() <= 7);
	}

	#[crate::integration_test]
	async fn register_player(ctx: &Context) {
		let player = NewPlayer {
			name: String::from("AlphaKeks"),
			steam_id: SteamID::from_u64(76561198282622073_u64).unwrap(),
			ip_address: Ipv6Addr::LOCALHOST,
		};

		let missing_auth_header = ctx
			.http_client
			.post(ctx.url("/players"))
			.json(&player)
			.send()
			.await?;

		assert_eq!(missing_auth_header.status(), 400);

		let jwt = ctx.auth_server(Duration::from_secs(0))?;

		sleep(Duration::from_secs(1)).await;

		let unauthorized = ctx
			.http_client
			.post(ctx.url("/players"))
			.header("Authorization", format!("Bearer {jwt}"))
			.json(&player)
			.send()
			.await?;

		assert_eq!(unauthorized.status(), 401);

		let jwt = ctx.auth_server(Duration::from_secs(60 * 60))?;

		let already_exists = ctx
			.http_client
			.post(ctx.url("/players"))
			.header("Authorization", format!("Bearer {jwt}"))
			.json(&player)
			.send()
			.await?;

		assert_eq!(already_exists.status(), 409);

		let player = NewPlayer {
			name: String::from("very cool person"),
			steam_id: SteamID::MAX,
			ip_address: Ipv6Addr::LOCALHOST,
		};

		let success = ctx
			.http_client
			.post(ctx.url("/players"))
			.header("Authorization", format!("Bearer {jwt}"))
			.json(&player)
			.send()
			.await?;

		assert_eq!(success.status(), 201);
	}
}
