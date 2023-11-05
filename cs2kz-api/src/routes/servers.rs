use {
	crate::{
		res::{servers as res, BadRequest},
		util::Created,
		Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetServersParams<'a> {
	name: Option<String>,
	owned_by: Option<PlayerIdentifier<'a>>,
	created_after: Option<DateTime<Utc>>,
	created_before: Option<DateTime<Utc>>,
	offset: Option<u64>,
	limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers", params(GetServersParams), responses(
	(status = 200, body = Vec<Server>),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_servers(
	state: State,
	Query(GetServersParams { name, owned_by, created_after, created_before, offset, limit }): Query<
		GetServersParams<'_>,
	>,
) -> Response<Vec<res::Server>> {
	todo!();
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers/{ident}", params(
	("ident" = ServerIdentifier, Path, description = "The servers's ID or name")
), responses(
	(status = 200, body = Server),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_server(
	state: State,
	Path(ident): Path<ServerIdentifier<'_>>,
) -> Response<res::Server> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServer {
	name: String,
	owned_by: SteamID,

	#[schema(value_type = String)]
	ip: Ipv4Addr,

	port: u16,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServerWithId {
	id: u16,

	#[serde(flatten)]
	server: NewServer,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Servers", context_path = "/api/v0", path = "/servers", request_body = NewServer, responses(
	(status = 201, body = NewServerWithId),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn create_server(
	state: State,
	Json(NewServer { name, owned_by, ip, port }): Json<NewServer>,
) -> Result<Created<Json<NewServerWithId>>> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerUpdate {
	name: Option<String>,
	owned_by: Option<SteamID>,

	#[schema(value_type = Option<String>)]
	ip: Option<Ipv4Addr>,

	port: Option<u16>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Servers", context_path = "/api/v0", path = "/servers/{id}", request_body = ServerUpdate, params(
	("id" = u16, Path, description = "The server's ID")
), responses(
	(status = 200),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn update_server(
	state: State,
	Path(server_id): Path<u16>,
	Json(ServerUpdate { name, owned_by, ip, port }): Json<ServerUpdate>,
) -> Response<()> {
	todo!();
}
