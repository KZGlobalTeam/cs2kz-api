//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::SteamID;
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	DeleteKeyRequest,
	DeleteKeyResponse,
	Error,
	FetchServerRequest,
	FetchServerResponse,
	FetchServersRequest,
	FetchServersResponse,
	GenerateAccessTokenRequest,
	GenerateAccessTokenResponse,
	Host,
	RegisterServerRequest,
	RegisterServerResponse,
	ResetKeyRequest,
	ResetKeyResponse,
	ServerService,
	UpdateServerRequest,
	UpdateServerResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::session::user::Permissions;
use crate::services::auth::session::{authorization, SessionManagerLayer};
use crate::services::auth::Session;
use crate::services::servers::ServerID;
use crate::util::ServerIdentifier;

impl From<ServerService> for Router
{
	fn from(svc: ServerService) -> Self
	{
		let admin_only = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::RequiredPermissions(Permissions::SERVERS),
			));

		let owner_auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::IsServerOwner::new(svc.database.clone()),
			));

		let no_cors = Router::new()
			.route("/auth", routing::post(generate_access_token))
			.with_state(svc.clone());

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:server", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/", routing::post(register_server).route_layer(admin_only.clone()))
			.route("/:server", routing::patch(update_server).route_layer(owner_auth.clone()))
			.route("/:server/key", routing::put(reset_api_key).route_layer(owner_auth.clone()))
			.route("/:server/key", routing::delete(delete_api_key).route_layer(admin_only.clone()))
			.route_layer(middleware::cors::dashboard([
				http::Method::OPTIONS,
				http::Method::POST,
				http::Method::PATCH,
				http::Method::PUT,
				http::Method::DELETE,
			]))
			.with_state(svc.clone());

		no_cors.merge(public).merge(protected)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/servers",
	tag = "Servers",
	operation_id = "get_servers",
	params(FetchServersRequest)
)]
async fn get_many(
	State(svc): State<ServerService>,
	Query(req): Query<FetchServersRequest>,
) -> Result<FetchServersResponse, ProblemDetails>
{
	let res = svc.fetch_servers(req).await?;

	if res.servers.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/servers", tag = "Servers", security(("Browser serssion" = ["servers"])))]
async fn register_server(
	session: Session,
	State(svc): State<ServerService>,
	Json(req): Json<RegisterServerRequest>,
) -> Result<RegisterServerResponse, ProblemDetails>
{
	let res = svc.register_server(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/servers/auth", tag = "Servers")]
async fn generate_access_token(
	State(svc): State<ServerService>,
	Json(req): Json<GenerateAccessTokenRequest>,
) -> Result<GenerateAccessTokenResponse, ProblemDetails>
{
	let res = svc.generate_access_token(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/servers/{server}", tag = "Servers", operation_id = "get_server", params(
  ("server" = ServerIdentifier, Path, description = "a server's ID or name"),
))]
async fn get_single(
	State(svc): State<ServerService>,
	Path(identifier): Path<ServerIdentifier>,
) -> Result<FetchServerResponse, ProblemDetails>
{
	let req = FetchServerRequest { identifier };
	let res = svc
		.fetch_server(req)
		.await?
		.ok_or(Error::ServerDoesNotExist)?;

	Ok(res)
}

/// Request payload for `PATCH /servers/{server}`
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UpdateServerRequest")]
#[doc(hidden)]
pub(crate) struct UpdateServerRequestPayload
{
	/// A new name.
	pub new_name: Option<String>,

	/// A new host.
	pub new_host: Option<Host>,

	/// A new port.
	pub new_port: Option<u16>,

	/// SteamID of a new owner.
	pub new_owner: Option<SteamID>,
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/servers/{server_id}",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn update_server(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
	Json(UpdateServerRequestPayload { new_name, new_host, new_port, new_owner }): Json<
		UpdateServerRequestPayload,
	>,
) -> Result<UpdateServerResponse, ProblemDetails>
{
	let req = UpdateServerRequest { server_id, new_name, new_host, new_port, new_owner };
	let res = svc.update_server(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  put,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn reset_api_key(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<ResetKeyResponse, ProblemDetails>
{
	let req = ResetKeyRequest { server_id };
	let res = svc.reset_key(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  delete,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn delete_api_key(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<DeleteKeyResponse, ProblemDetails>
{
	let req = DeleteKeyRequest { server_id };
	let res = svc.delete_key(req).await?;

	Ok(res)
}
