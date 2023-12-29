//! This module holds all HTTP handlers related to servers.

use std::net::{Ipv4Addr, SocketAddrV4};

use axum::extract::{Path, Query};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID};
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use utoipa::{IntoParams, ToSchema};

use crate::models::Server;
use crate::permissions::Permissions;
use crate::responses::Created;
use crate::sql::FetchID;
use crate::{openapi as R, sql, AppState, Error, Result, State};

static GET_BASE_QUERY: &str = r#"
	SELECT
		s.id,
		s.name,
		s.ip_address,
		s.port,
		o.steam_id owner_steam_id,
		o.name owner_name,
		s.approved_on
	FROM
		Servers s
		JOIN Players o ON o.steam_id = s.owned_by
"#;

/// This function returns the router for the `/servers` routes.
pub fn router(state: &'static AppState) -> Router {
	let add_server = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_web_user::<{ Permissions::SERVERS_ADD.0 }>,
	);

	let edit_server = axum::middleware::from_fn_with_state(
		state,
		crate::middleware::auth::verify_web_user::<{ Permissions::SERVERS_EDIT.0 }>,
	);

	Router::new()
		.route("/", get(get_servers))
		.route("/", post(create_server).layer(add_server))
		.route("/:ident", get(get_server_by_ident))
		.route("/:ident", patch(update_server).layer(edit_server))
		.with_state(state)
}

/// This endpoint allows you to fetch servers.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Servers",
	path = "/servers",
	params(GetServersParams),
	responses(
		R::Ok<Server>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_servers(
	state: State,
	Query(params): Query<GetServersParams<'_>>,
) -> Result<Json<Vec<Server>>> {
	let mut query = QueryBuilder::new(GET_BASE_QUERY);
	let mut filter = sql::Filter::new();

	if let Some(name) = params.name {
		query
			.push(filter)
			.push(" s.name LIKE ")
			.push_bind(format!("%{name}%)"));

		filter.switch();
	}

	if let Some(player_ident) = params.owned_by {
		let steam_id = player_ident.fetch_id(state.database()).await?;

		query
			.push(filter)
			.push(" s.owned_by = ")
			.push_bind(steam_id);

		filter.switch();
	}

	if let Some(approved) = params.approved {
		query.push(filter).push(" s.api_key IS ");

		if approved {
			query.push(" NOT ");
		}

		query.push(" NULL ");
		filter.switch();
	}

	sql::push_limits::<500>(params.limit, params.offset, &mut query);

	let servers = query
		.build_query_as::<Server>()
		.fetch_all(state.database())
		.await?;

	if servers.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(servers))
}

/// This endpoint is used for creating new servers.
///
/// It is intended to be used by admins and one-time-use tokens given to players.
#[tracing::instrument]
#[utoipa::path(
	post,
	tag = "Servers",
	path = "/servers",
	request_body = CreateServerRequest,
	responses(
		R::Created<CreateServerResponse>,
		R::BadRequest,
		R::Conflict,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn create_server(
	state: State,
	Json(body): Json<CreateServerRequest>,
) -> Result<Created<Json<CreateServerResponse>>> {
	let mut transaction = state.begin_transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Servers (name, ip_address, port, owned_by)
		VALUES
			(?, ?, ?, ?)
		"#,
		body.name,
		body.ip_address.ip().to_string(),
		body.ip_address.port(),
		body.owned_by,
	}
	.execute(transaction.as_mut())
	.await?;

	let server_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await?
		.id as _;

	Ok(Created(Json(CreateServerResponse { server_id })))
}

/// This endpoint allows you to fetch a single server by its ID or (parts of its) name.
#[tracing::instrument]
#[utoipa::path(
	get,
	tag = "Servers",
	path = "/servers/{ident}",
	params(("ident" = ServerIdentifier<'_>, Path, description = "A server's ID or name.")),
	responses(
		R::Ok<Server>,
		R::NoContent,
		R::BadRequest,
		R::InternalServerError,
	),
)]
pub async fn get_server_by_ident(
	state: State,
	Path(ident): Path<ServerIdentifier<'_>>,
) -> Result<Json<Server>> {
	let mut query = QueryBuilder::new(format!("{GET_BASE_QUERY} WHERE"));

	match ident {
		ServerIdentifier::ID(id) => {
			query.push(" s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query.push(" s.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	query
		.build_query_as::<Server>()
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::NoContent)
		.map(Json)
}

/// This endpoint allows you to update a single server by its ID.
#[tracing::instrument]
#[utoipa::path(
	patch,
	tag = "Servers",
	path = "/servers/{id}",
	params(("id" = u16, Path, description = "A server's ID.")),
	request_body = UpdateServerRequest,
	responses(
		R::Ok,
		R::BadRequest,
		R::Unauthorized,
		R::InternalServerError,
	),
)]
pub async fn update_server(
	state: State,
	Path(server_id): Path<u16>,
	Json(body): Json<UpdateServerRequest>,
) -> Result<()> {
	let mut update_server = QueryBuilder::new("UPDATE Servers");
	let mut delimiter = " SET ";

	if let Some(name) = body.name {
		update_server
			.push(delimiter)
			.push(" name = ")
			.push_bind(name);

		delimiter = ",";
	}

	if let Some(ip_address) = body.ip_address {
		update_server
			.push(delimiter)
			.push(" ip_address = ")
			.push_bind(ip_address.to_string());

		delimiter = ",";
	}

	if let Some(port) = body.port {
		update_server
			.push(delimiter)
			.push(" port = ")
			.push_bind(port);

		delimiter = ",";
	}

	if let Some(steam_id) = body.owned_by {
		update_server
			.push(delimiter)
			.push(" owned_by = ")
			.push_bind(steam_id);
	}

	update_server.build().execute(state.database()).await?;

	Ok(())
}

/// Query parameters for retrieving information about servers.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetServersParams<'a> {
	/// The name of the server.
	name: Option<String>,

	/// Only include servers owned by this player.
	owned_by: Option<PlayerIdentifier<'a>>,

	/// Only include servers that are (not) approved (have an API key).
	approved: Option<bool>,

	#[param(minimum = 0, maximum = 500)]
	limit: Option<u64>,
	offset: Option<i64>,
}

/// A new server.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "name": "Alpha's KZ",
  "ip_address": "255.255.255.255:1337",
  "owned_by": "STEAM_1:1:161178172"
}))]
pub struct CreateServerRequest {
	/// The server's name.
	name: String,

	/// The server's IP address and port.
	#[schema(value_type = String)]
	ip_address: SocketAddrV4,

	/// The SteamID of the player who owns this server.
	owned_by: SteamID,
}

/// A server update.
#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
  "name": "Alpha's KZ",
  "ip_address": "255.255.255.255",
  "port": 1337,
  "owned_by": "STEAM_1:1:161178172"
}))]
pub struct UpdateServerRequest {
	/// The server's new name.
	name: Option<String>,

	/// The server's new IP address.
	#[schema(value_type = String)]
	ip_address: Option<Ipv4Addr>,

	/// The server's new port.
	port: Option<u16>,

	/// The server's new owner.
	owned_by: Option<SteamID>,
}

/// A newly created server.
#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({ "server_id": 1 }))]
pub struct CreateServerResponse {
	/// The server's ID.
	server_id: u16,
}
