//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::SteamID;
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	AdminService,
	Error,
	FetchAdminRequest,
	FetchAdminResponse,
	FetchAdminsRequest,
	FetchAdminsResponse,
	SetPermissionsRequest,
	SetPermissionsResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::session::authorization::RequiredPermissions;
use crate::services::auth::session::user::Permissions;
use crate::services::auth::session::SessionManagerLayer;
use crate::services::auth::Session;

impl From<AdminService> for Router
{
	fn from(svc: AdminService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				RequiredPermissions(Permissions::ADMIN),
			));

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:steam_id", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/:steam_id", routing::put(set_permissions).route_layer(auth))
			.route_layer(middleware::cors::dashboard([http::Method::OPTIONS, http::Method::PUT]))
			.with_state(svc.clone());

		public.merge(protected)
	}
}

/// Fetch many bans.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(
	get,
	path = "/admins",
	tag = "Admins",
	operation_id = "get_admins",
	params(FetchAdminsRequest)
)]
async fn get_many(
	State(svc): State<AdminService>,
	Query(req): Query<FetchAdminsRequest>,
) -> Result<FetchAdminsResponse, ProblemDetails>
{
	let res = svc.fetch_admins(req).await?;

	if res.admins.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(res)
}

/// Fetch a specific ban by its ID.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/admins/{admin_id}", tag = "Admins", operation_id = "get_admin", params(
  ("admin_id" = SteamID, Path, description = "an admin's SteamID"),
))]
async fn get_single(
	State(svc): State<AdminService>,
	Path(user_id): Path<SteamID>,
) -> Result<FetchAdminResponse, ProblemDetails>
{
	let req = FetchAdminRequest { user_id };
	let res = svc
		.fetch_admin(req)
		.await?
		.ok_or(super::Error::UserDoesNotExist { user_id })?;

	Ok(res)
}

/// Request payload for the `set_permissions` handler.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "SetPermissionsRequest")]
#[doc(hidden)]
pub(crate) struct SetPermissionsPayload
{
	/// The permissions to set for the user.
	permissions: Permissions,
}

/// Set a user's permissions.
#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
#[utoipa::path(
  put,
  path = "/admins/{admin_id}",
  tag = "Admins",
  operation_id = "update_admin",
  params(("admin_id" = SteamID, Path, description = "an admin's SteamID")),
  security(("Browser Session" = ["admin"])),
)]
async fn set_permissions(
	session: Session,
	State(svc): State<AdminService>,
	Path(user_id): Path<SteamID>,
	Json(SetPermissionsPayload { permissions }): Json<SetPermissionsPayload>,
) -> Result<SetPermissionsResponse, ProblemDetails>
{
	let req = SetPermissionsRequest { user_id, permissions };
	let res = svc.set_permissions(req).await?;

	Ok(res)
}
