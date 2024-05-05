//! Handlers for the `/servers/{server}` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::ServerIdentifier;
use sqlx::QueryBuilder;
use tracing::info;

use crate::responses::NoContent;
use crate::servers::{queries, Server, ServerID, ServerUpdate};
use crate::sqlx::UpdateQuery;
use crate::{auth, responses, Error, Result, State};

/// Fetch a specific server.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  path = "/servers/{server}",
  tag = "Servers",
  responses(
    responses::Ok<Server>,
    responses::NoContent,
    responses::BadRequest,
    responses::InternalServerError,
  ),
)]
pub async fn get(state: &State, Path(server): Path<ServerIdentifier>) -> Result<Json<Server>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	match server {
		ServerIdentifier::ID(id) => {
			query.push(" s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query.push(" s.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let server = query
		.build_query_as::<Server>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::no_content())?;

	Ok(Json(server))
}

/// Update a server.
///
/// This endpoint can be used by both admins and server owners.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  path = "/servers/{server}",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  responses(//
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
)]
pub async fn patch(
	state: &State,
	session: auth::Session<auth::AdminOrServerOwner>,
	Path(server_id): Path<ServerID>,
	Json(ServerUpdate { name, ip_address, owned_by }): Json<ServerUpdate>,
) -> Result<NoContent> {
	if name.is_none() && ip_address.is_none() && owned_by.is_none() {
		return Ok(NoContent);
	}

	let mut transaction = state.transaction().await?;
	let mut query = UpdateQuery::new("Servers");

	if let Some(name) = name {
		query.set("name", name);
	}

	if let Some(ip_address) = ip_address {
		query
			.set("ip_address", ip_address.ip().to_string())
			.set("port", ip_address.port());
	}

	if let Some(steam_id) = owned_by {
		query.set("owner_id", steam_id);
	}

	query.push(" WHERE id = ").push_bind(server_id);

	let query_result = query.build().execute(transaction.as_mut()).await?;

	if query_result.rows_affected() == 0 {
		return Err(Error::unknown("server ID"));
	}

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, session.user = ?session.user(), "updated server");

	Ok(NoContent)
}

#[cfg(test)]
mod tests {
	use axum_extra::extract::cookie::Cookie;
	use cs2kz::SteamID;
	use reqwest::header;

	use crate::servers::{Server, ServerUpdate};

	#[crate::test]
	async fn fetch_server(ctx: &Context) {
		let response = ctx
			.http_client
			.get(ctx.url("/servers/alpha"))
			.send()
			.await?;

		assert_eq!(response.status(), 200);

		let server = response.json::<Server>().await?;

		assert_eq!(server.name, "Alpha's KZ");
		assert_eq!(server.owner.steam_id, 76561198282622073_u64);
	}

	#[crate::test]
	async fn update_server(ctx: &Context) {
		let update = ServerUpdate {
			name: Some(String::from("Church of Schnose")),
			ip_address: None,
			owned_by: None,
		};

		let server = ctx
			.http_client
			.get(ctx.url("/servers/1"))
			.send()
			.await?
			.json::<Server>()
			.await?;

		assert_eq!(server.name, "Alpha's KZ");

		let url = ctx.url(format_args!("/servers/{}", server.id));
		let response = ctx
			.http_client
			.patch(url.clone())
			.json(&update)
			.send()
			.await?;

		assert_eq!(response.status(), 401);

		let alphakeks = SteamID::from_u64(76561198282622073_u64)?;
		let session = ctx.auth_session(alphakeks).await?;
		let session_cookie = Cookie::from(session).encoded().to_string();

		let response = ctx
			.http_client
			.patch(url)
			.header(header::COOKIE, session_cookie)
			.json(&update)
			.send()
			.await?;

		assert_eq!(response.status(), 204);

		let server = ctx
			.http_client
			.get(ctx.url("/servers/1"))
			.send()
			.await?
			.json::<Server>()
			.await?;

		assert_eq!(server.name, "Church of Schnose");
	}
}
