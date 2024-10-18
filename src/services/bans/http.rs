//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::SteamID;
use serde::Deserialize;
use time::OffsetDateTime;
use tower::ServiceBuilder;

use super::models::UnbanReason;
use super::{
	BanID,
	BanReason,
	BanRequest,
	BanResponse,
	BanService,
	BannedBy,
	Error,
	FetchBanRequest,
	FetchBanResponse,
	FetchBansRequest,
	FetchBansResponse,
	UnbanRequest,
	UnbanResponse,
	UpdateBanRequest,
	UpdateBanResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::net::IpAddr;
use crate::services::auth::session::authorization::RequiredPermissions;
use crate::services::auth::session::user::Permissions;
use crate::services::auth::session::SessionManagerLayer;
use crate::services::auth::{jwt, Jwt, Session};

impl From<BanService> for Router
{
	fn from(svc: BanService) -> Self
	{
		let session_auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				RequiredPermissions(Permissions::BANS),
			));

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:id", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/", routing::post(create).route_layer(session_auth.clone()))
			.route("/:id", routing::patch(update).route_layer(session_auth.clone()))
			.route("/:id", routing::delete(revert).route_layer(session_auth.clone()))
			.route_layer(middleware::cors::dashboard([
				http::Method::OPTIONS,
				http::Method::POST,
				http::Method::PATCH,
				http::Method::DELETE,
			]))
			.with_state(svc.clone());

		public.merge(protected)
	}
}

/// Fetch many bans.
#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/bans",
	tag = "Bans",
	operation_id = "get_bans",
	params(FetchBansRequest)
)]
async fn get_many(
	State(svc): State<BanService>,
	Query(req): Query<FetchBansRequest>,
) -> Result<FetchBansResponse, ProblemDetails>
{
	let res = svc.fetch_bans(req).await?;

	if res.bans.is_empty() {
		Err(Error::NoData)?;
	}

	Ok(res)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "BanRequest")]
#[doc(hidden)]
pub(crate) struct BanRequestPayload
{
	/// The player's SteamID.
	pub player_id: SteamID,

	/// The player's IP address.
	pub player_ip: Option<IpAddr>,

	/// The reason for the ban.
	pub reason: BanReason,
}

/// Ban a player.
#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/bans", tag = "Bans", operation_id = "submit_ban", security(
  ("CS2 Server" = []),
  ("Browser Session" = ["bans"]),
))]
async fn create(
	server: Option<Jwt<jwt::ServerInfo>>,
	session: Option<Session>,
	State(svc): State<BanService>,
	Json(BanRequestPayload { player_id, player_ip, reason }): Json<BanRequestPayload>,
) -> Result<BanResponse, ProblemDetails>
{
	let banned_by = match (server, session) {
		(None, None) => {
			return Err(Error::Unauthorized)?;
		}
		(Some(server), Some(session)) => {
			tracing::warn!(?server, ?session, "ban request is doubly authorized");

			return Err(Error::DoublyAuthorized {
				server_id: server.id(),
				admin_id: session.user().steam_id(),
			})?;
		}
		(Some(server), None) => {
			BannedBy::Server { id: server.id(), plugin_version_id: server.plugin_version_id() }
		}
		(None, Some(session)) => {
			if !session.user().permissions().contains(Permissions::BANS) {
				Err(Error::Unauthorized)?;
			}

			BannedBy::Admin { steam_id: session.user().steam_id() }
		}
	};

	let req = BanRequest { player_id, player_ip, reason, banned_by };
	let res = svc.ban_player(req).await?;

	Ok(res)
}

/// Fetch a specific ban by its ID.
#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/bans/{ban_id}", tag = "Bans", operation_id = "get_ban", params(
  ("ban_id" = BanID, Path, description = "a ban's ID"),
))]
async fn get_single(
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
) -> Result<FetchBanResponse, ProblemDetails>
{
	let res = svc
		.fetch_ban(FetchBanRequest { ban_id })
		.await?
		.ok_or(Error::BanDoesNotExist { ban_id })?;

	Ok(res)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UpdateBanRequest")]
#[doc(hidden)]
pub(crate) struct UpdateBanRequestPayload
{
	/// A new ban reason.
	new_reason: Option<String>,

	/// A new expiration date.
	#[serde(default, with = "time::serde::rfc3339::option")]
	new_expiration_date: Option<OffsetDateTime>,
}

/// Update a ban.
#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/bans/{ban_id}",
  tag = "Bans",
  operation_id = "update_ban",
  params(("ban_id" = BanID, Path, description = "a ban's ID")),
  security(("Browser Session" = ["bans"])),
)]
async fn update(
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(UpdateBanRequestPayload { new_reason, new_expiration_date }): Json<
		UpdateBanRequestPayload,
	>,
) -> Result<UpdateBanResponse, ProblemDetails>
{
	let req = UpdateBanRequest { ban_id, new_reason, new_expiration_date };
	let res = svc.update_ban(req).await?;

	Ok(res)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UnbanRequest")]
#[doc(hidden)]
pub(crate) struct UnbanRequestPayload
{
	/// The reason for the unban.
	#[schema(value_type = str)]
	reason: UnbanReason,
}

/// Unban a player.
#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  delete,
  path = "/bans/{ban_id}",
  tag = "Bans",
  operation_id = "revert_ban",
  params(("ban_id" = BanID, Path, description = "a ban's ID")),
  security(("Browser Session" = ["bans"])),
)]
async fn revert(
	session: Session,
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(UnbanRequestPayload { reason }): Json<UnbanRequestPayload>,
) -> Result<UnbanResponse, ProblemDetails>
{
	let req = UnbanRequest { ban_id, reason, admin_id: session.user().steam_id() };
	let res = svc.unban_player(req).await?;

	Ok(res)
}
