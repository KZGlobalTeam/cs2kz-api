//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};
use axum_extra::extract::Query;
use cs2kz::SteamID;
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	Error,
	FetchPlayerPreferencesRequest,
	FetchPlayerPreferencesResponse,
	FetchPlayerRequest,
	FetchPlayerResponse,
	FetchPlayersRequest,
	FetchPlayersResponse,
	FetchSteamProfileResponse,
	PlayerService,
	RegisterPlayerRequest,
	RegisterPlayerResponse,
	UpdatePlayerRequest,
	UpdatePlayerResponse,
};
use crate::http::extract::{Json, Path};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::net::IpAddr;
use crate::services::auth::jwt::{self, JwtLayer};
use crate::services::auth::session::user::Permissions;
use crate::services::auth::{Jwt, Session};
use crate::util::PlayerIdentifier;

impl From<PlayerService> for Router
{
	fn from(svc: PlayerService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(JwtLayer::<jwt::ServerInfo>::new(svc.auth_svc.clone()));

		Router::new()
			.route("/", routing::get(get_many))
			.route("/", routing::post(register_player).route_layer(auth.clone()))
			.route("/:player", routing::get(get_single))
			.route("/:player", routing::patch(update_player).route_layer(auth))
			.route("/:player/preferences", routing::get(get_preferences))
			.route("/:player/steam", routing::get(get_steam_profile))
			.with_state(svc)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/players", tag = "Players", params(FetchPlayersRequest))]
async fn get_many(
	session: Option<Session>,
	State(svc): State<PlayerService>,
	Query(req): Query<FetchPlayersRequest>,
) -> Result<FetchPlayersResponse, ProblemDetails>
{
	let may_view_ips =
		session.is_some_and(|session| session.user().permissions().contains(Permissions::BANS));

	let mut res = svc.fetch_players(req).await?;

	if res.players.is_empty() {
		Err(Error::NoData)?;
	}

	if !may_view_ips {
		for player in &mut res.players {
			player.ip_address = None;
		}
	}

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/players", tag = "Players", security(
  ("CS2 Server" = []),
))]
async fn register_player(
	server: Jwt<jwt::ServerInfo>,
	State(svc): State<PlayerService>,
	Json(req): Json<RegisterPlayerRequest>,
) -> Result<RegisterPlayerResponse, ProblemDetails>
{
	let res = svc.register_player(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/players/{player}", tag = "Players", params(
  ("player" = PlayerIdentifier, Path, description = "a player's SteamID or name"),
))]
async fn get_single(
	session: Option<Session>,
	State(svc): State<PlayerService>,
	Path(identifier): Path<PlayerIdentifier>,
) -> Result<FetchPlayerResponse, ProblemDetails>
{
	let may_view_ips =
		session.is_some_and(|session| session.user().permissions().contains(Permissions::BANS));

	let mut player = svc
		.fetch_player(FetchPlayerRequest { identifier })
		.await?
		.ok_or(Error::PlayerDoesNotExist)?;

	if !may_view_ips {
		player.ip_address = None;
	}

	Ok(player)
}

/// Request payload for `PATCH /players/{player}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[cfg_attr(test, derive(serde::Serialize))]
#[schema(title = "UpdatePlayerRequest")]
#[doc(hidden)]
pub(crate) struct UpdatePlayerPayload
{
	/// The player's current name.
	pub name: String,

	/// The player's current IP address.
	pub ip_address: IpAddr,

	/// The player's current in-game preferences.
	#[schema(value_type = Object)]
	pub preferences: serde_json::Value,

	/// The player's in-game session.
	#[schema(value_type = Session)]
	pub session: super::Session,
}

#[tracing::instrument(skip(server), fields(server.id = %server.id()), err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/players/{player_id}",
  tag = "Players",
  params(("player_id" = SteamID, Path, description = "a player's SteamID")),
  security(("CS2 Server" = [])),
)]
async fn update_player(
	server: Jwt<jwt::ServerInfo>,
	State(svc): State<PlayerService>,
	Path(player_id): Path<SteamID>,
	Json(UpdatePlayerPayload { name, ip_address, preferences, session }): Json<UpdatePlayerPayload>,
) -> Result<UpdatePlayerResponse, ProblemDetails>
{
	let req = UpdatePlayerRequest {
		player_id,
		server_id: server.id(),
		name,
		ip_address,
		preferences,
		session,
	};

	let res = svc.update_player(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/players/{player}/preferences", tag = "Players", params(
  ("player" = PlayerIdentifier, Path, description = "a player's SteamID or name"),
))]
async fn get_preferences(
	State(svc): State<PlayerService>,
	Path(identifier): Path<PlayerIdentifier>,
) -> Result<FetchPlayerPreferencesResponse, ProblemDetails>
{
	let req = FetchPlayerPreferencesRequest { identifier };
	let res = svc
		.fetch_player_preferences(req)
		.await?
		.ok_or(Error::NoData)?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/players/{player_id}/steam", tag = "Players", params(
  ("player_id" = SteamID, Path, description = "a player's SteamID"),
))]
async fn get_steam_profile(
	State(svc): State<PlayerService>,
	Path(player_id): Path<SteamID>,
) -> Result<FetchSteamProfileResponse, ProblemDetails>
{
	let res = svc
		.steam_svc
		.fetch_user(player_id)
		.await
		.map(FetchSteamProfileResponse)?;

	Ok(res)
}

#[cfg(test)]
mod tests
{
	use std::time::Duration;

	use axum::extract::Request;
	use axum::handler::Handler;
	use fake::{Fake, Faker};
	use serde_json::json;
	use sqlx::{MySql, Pool};
	use tower::Service;

	use super::*;
	use crate::services::auth::jwt;
	use crate::services::auth::session::SessionID;
	use crate::testing;

	const ALPHAKEKS_ID: SteamID = match SteamID::new(76561198282622073_u64) {
		Some(id) => id,
		None => unreachable!(),
	};

	#[sqlx::test(migrations = "database/migrations")]
	async fn get_many_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database);
		let handler = routing::get(get_many);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(Default::default())?;

		let res = handler.call(req, state).await;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchPlayersResponse>(res.into_body()).await?;

		testing::assert!(!res.players.is_empty());
		testing::assert_eq!(res.players.len() as u64, res.total);

		for player in res.players {
			testing::assert!(player.ip_address.is_none());
		}

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/session.sql")
	)]
	async fn get_many_with_auth_returns_ips(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database);
		let handler = routing::get(get_many);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.header("Cookie", format!("kz-auth={}", SessionID::TESTING))
			.body(Default::default())?;

		let res = handler.call(req, state).await;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchPlayersResponse>(res.into_body()).await?;

		testing::assert!(!res.players.is_empty());
		testing::assert_eq!(res.players.len() as u64, res.total);

		for player in res.players {
			testing::assert!(player.ip_address.is_some());
		}

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn get_single_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database);
		let mut handler = Router::new()
			.route("/:player", routing::get(get_single))
			.with_state(state);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/alphakeks")
			.body(axum::body::Body::default())?;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchPlayerResponse>(res.into_body()).await?;

		testing::assert_eq!(res.info.name, "AlphaKeks");
		testing::assert_eq!(res.info.steam_id, ALPHAKEKS_ID);
		testing::assert!(res.ip_address.is_none());

		Ok(())
	}

	#[sqlx::test(
		migrations = "database/migrations",
		fixtures("../../../database/fixtures/session.sql")
	)]
	async fn get_single_with_auth_returns_ip(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database);
		let mut handler = Router::new()
			.route("/:player", routing::get(get_single))
			.with_state(state);

		let req = Request::builder()
			.method(http::Method::GET)
			.uri("/alphakeks")
			.header("Cookie", format!("kz-auth={}", SessionID::TESTING))
			.body(axum::body::Body::default())?;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::OK);

		let res = testing::parse_body::<FetchPlayerResponse>(res.into_body()).await?;

		testing::assert_eq!(res.info.name, "AlphaKeks");
		testing::assert_eq!(res.info.steam_id, ALPHAKEKS_ID);
		testing::assert!(res.ip_address.is_some());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_player_works(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database.clone());
		let mut handler = Router::new()
			.route("/players/:id", routing::patch(update_player))
			.with_state(state);

		let auth_svc = testing::auth_svc(database);
		let info = jwt::ServerInfo::new(1.into(), 1.into());
		let expires_after = Duration::from_secs(69);
		let jwt = auth_svc.encode_jwt(Jwt::new(&info, expires_after))?;
		let body = serde_json::to_string(&UpdatePlayerPayload {
			name: String::from("(͡ ͡° ͜ つ ͡͡°)"),
			ip_address: "::1".parse()?,
			preferences: json!({ "foo": "bar" }),
			session: Faker.fake(),
		})?;

		let req = Request::builder()
			.method(http::Method::PATCH)
			.uri(format!("/players/{ALPHAKEKS_ID}"))
			.header("Authorization", format!("Bearer {jwt}"))
			.header("Content-Type", "application/json")
			.body(axum::body::Body::from(body))?;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::CREATED);

		let res = testing::parse_body::<UpdatePlayerResponse>(res.into_body()).await?;

		testing::assert!(res.course_session_ids.is_empty());

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_player_not_found(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database.clone());
		let mut handler = Router::new()
			.route("/players/:id", routing::patch(update_player))
			.with_state(state);

		let auth_svc = testing::auth_svc(database);
		let info = jwt::ServerInfo::new(1.into(), 1.into());
		let expires_after = Duration::from_secs(69);
		let jwt = auth_svc.encode_jwt(Jwt::new(&info, expires_after))?;
		let body = serde_json::to_string(&UpdatePlayerPayload {
			name: String::from("(͡ ͡° ͜ つ ͡͡°)"),
			ip_address: "::1".parse()?,
			preferences: json!({ "foo": "bar" }),
			session: Faker.fake(),
		})?;

		let steam_id = const {
			match SteamID::new(76561198264939817) {
				Some(id) => id,
				None => unreachable!(),
			}
		};

		let req = Request::builder()
			.method(http::Method::PATCH)
			.uri(format!("/players/{steam_id}"))
			.header("Authorization", format!("Bearer {jwt}"))
			.header("Content-Type", "application/json")
			.body(axum::body::Body::from(body))?;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::NOT_FOUND);

		Ok(())
	}

	#[sqlx::test(migrations = "database/migrations")]
	async fn update_player_unauthorized(database: Pool<MySql>) -> color_eyre::Result<()>
	{
		let state = testing::player_svc(database.clone());
		let mut handler = Router::new()
			.route("/players/:id", routing::patch(update_player))
			.with_state(state);

		let auth_svc = testing::auth_svc(database);
		let info = jwt::ServerInfo::new(1.into(), 1.into());
		let expires_after = Duration::from_secs(1);
		let jwt = auth_svc.encode_jwt(Jwt::new(&info, expires_after))?;
		let body = serde_json::to_string(&UpdatePlayerPayload {
			name: String::from("(͡ ͡° ͜ つ ͡͡°)"),
			ip_address: "::1".parse()?,
			preferences: json!({ "foo": "bar" }),
			session: Faker.fake(),
		})?;

		let req = Request::builder()
			.method(http::Method::PATCH)
			.uri(format!("/players/{ALPHAKEKS_ID}"))
			.header("Authorization", format!("Bearer {jwt}"))
			.header("Content-Type", "application/json")
			.body(axum::body::Body::from(body))?;

		tokio::time::sleep(Duration::from_secs(2)).await;

		let res = handler.call(req).await?;

		testing::assert_eq!(res.status(), http::StatusCode::UNAUTHORIZED);

		Ok(())
	}
}
