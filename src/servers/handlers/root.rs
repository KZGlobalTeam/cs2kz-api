//! Handlers for the `/servers` route.

use std::net::Ipv4Addr;

use axum::extract::Query;
use axum::Json;
use chrono::{DateTime, Utc};
use cs2kz::PlayerIdentifier;
use serde::Deserialize;
use tracing::debug;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::auth::RoleFlags;
use crate::parameters::{Limit, Offset};
use crate::responses::Created;
use crate::servers::{queries, CreatedServer, NewServer, Server, ServerID};
use crate::sqlx::{FetchID, FilteredQuery, QueryBuilderExt, SqlErrorExt};
use crate::{auth, responses, Error, Result, State};

/// Query parameters for `GET /servers`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetParams {
	/// Filter by name.
	name: Option<String>,

	/// Filter by IP address.
	ip_address: Option<Ipv4Addr>,

	/// Filter by server owner.
	owned_by: Option<PlayerIdentifier>,

	/// Filter by creation date.
	created_after: Option<DateTime<Utc>>,

	/// Filter by creation date.
	created_before: Option<DateTime<Utc>>,

	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/servers",
  tag = "Servers",
  params(GetParams),
  responses(
    responses::Ok<Server>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(
	state: &'static State,
	Query(GetParams {
		name,
		ip_address,
		owned_by,
		created_after,
		created_before,
		limit,
		offset,
	}): Query<GetParams>,
) -> Result<Json<Vec<Server>>> {
	let mut query = FilteredQuery::new(queries::SELECT);

	if let Some(name) = name {
		query.filter(" s.name LIKE ", format!("%{name}%"));
	}

	if let Some(ip_address) = ip_address {
		query.filter(" s.ip_address = ", ip_address.to_string());
	}

	if let Some(player) = owned_by {
		let steam_id = player.fetch_id(&state.database).await?;

		query.filter(" s.owner_id = ", steam_id);
	}

	if let Some(created_after) = created_after {
		query.filter(" s.created_on > ", created_after);
	}

	if let Some(created_before) = created_before {
		query.filter(" s.created_on < ", created_before);
	}

	query.push_limits(limit, offset);

	let servers = query
		.build_query_as::<Server>()
		.fetch_all(&state.database)
		.await?;

	if servers.is_empty() {
		return Err(Error::no_content());
	}

	Ok(Json(servers))
}

#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  path = "/servers",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  responses(
    responses::Created<CreatedServer>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn post(
	state: &'static State,
	session: auth::Session<auth::HasRoles<{ RoleFlags::SERVERS.as_u32() }>>,
	Json(NewServer { name, ip_address, owned_by }): Json<NewServer>,
) -> Result<Created<Json<CreatedServer>>> {
	let mut transaction = state.transaction().await?;
	let refresh_key = Uuid::new_v4();
	let server_id = sqlx::query! {
		r#"
		INSERT INTO
		  Servers (name, ip_address, port, owner_id, refresh_key)
		VALUES
		  (?, ?, ?, ?, ?)
		"#,
		name,
		ip_address.ip().to_string(),
		ip_address.port(),
		owned_by,
		refresh_key,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("owner_id") {
			Error::unknown("owner").with_source(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.try_into()
	.map(ServerID)
	.map_err(Error::invalid_id_column)?;

	transaction.commit().await?;

	debug!(id = %server_id, %refresh_key, session.user = ?session.user(), "created new server");

	Ok(Created(Json(CreatedServer { server_id, refresh_key })))
}

#[cfg(test)]
mod tests {
	use std::net::{Ipv4Addr, SocketAddrV4};

	use axum_extra::extract::cookie::Cookie;
	use cs2kz::SteamID;
	use reqwest::header;

	use crate::servers::{CreatedServer, NewServer, Server};

	#[crate::test]
	async fn fetch_servers(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/servers"))
			.query(&[("limit", "7")])
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let servers = response.json::<Vec<Server>>().await?;

		assert!(servers.len() <= 7);
	}

	#[crate::test(fixtures = ["alphakeks-server-role"])]
	async fn approve_server(ctx: &Context) {
		let alphakeks = SteamID::from_u64(76561198282622073_u64)?;
		let server = NewServer {
			name: String::from("very cool server"),
			ip_address: SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 69),
			owned_by: alphakeks,
		};

		let response = ctx
			.http_client
			.post(ctx.url("/servers"))
			.json(&server)
			.send()
			.await?;

		assert_eq!(response.status(), 401);

		let session = ctx.auth_session(alphakeks).await?;
		let session_cookie = Cookie::from(session).encoded().to_string();

		let response = ctx
			.http_client
			.post(ctx.url("/servers"))
			.header(header::COOKIE, session_cookie)
			.json(&server)
			.send()
			.await?;

		assert_eq!(response.status(), 201);

		let CreatedServer { server_id, .. } = response.json().await?;

		let url = ctx.url(format_args!("/servers/{server_id}"));
		let server = ctx
			.http_client
			.get(url)
			.send()
			.await?
			.json::<Server>()
			.await?;

		assert_eq!(server.id, server_id);
		assert_eq!(server.name, "very cool server");
		assert_eq!(server.owner.steam_id, alphakeks);
	}
}
