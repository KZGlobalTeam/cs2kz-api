use {
	super::{BoundedU64, Filter},
	crate::{
		database,
		middleware::auth::gameservers::AuthenticatedServer,
		res::{player as res, responses, Created},
		Error, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Extension, Json,
	},
	cs2kz::{PlayerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

static ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		p.*,
		s.time_active,
		s.time_spectating,
		s.time_afk,
		s.perfs,
		s.bhops_tick0,
		s.bhops_tick1,
		s.bhops_tick2,
		s.bhops_tick3,
		s.bhops_tick4,
		s.bhops_tick5,
		s.bhops_tick6,
		s.bhops_tick7,
		s.bhops_tick8
	FROM
		Players p
		JOIN Sessions s ON s.player_id = p.steam_id
"#;

/// Query parameters for fetching players.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetPlayersParams {
	/// Name of a player.
	name: Option<String>,

	/// A minimum amount of playtime.
	playtime: Option<u32>,

	/// Only include (not) banned players.
	is_banned: Option<bool>,

	#[param(value_type = Option<u64>, default = 0)]
	offset: BoundedU64,

	/// Return at most this many results.
	#[param(value_type = Option<u64>, default = 100, maximum = 500)]
	limit: BoundedU64<100, 500>,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(get, tag = "Players", context_path = "/api", path = "/players",
	params(GetPlayersParams),
	responses(
		responses::Ok<res::Player>,
		responses::NoContent,
		responses::BadRequest,
		responses::InternalServerError,
	),
)]
pub async fn get_players(
	state: State,
	Query(GetPlayersParams { name, playtime, is_banned, offset, limit }): Query<GetPlayersParams>,
) -> Result<Json<Vec<res::Player>>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = Filter::new();

	if let Some(name) = name {
		query
			.push(filter)
			.push(" p.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(playtime) = playtime {
		query
			.push(filter)
			.push(" s.time_active >= ")
			.push_bind(playtime);

		filter.switch();
	}

	if let Some(is_banned) = is_banned {
		query
			.push(filter)
			.push(" p.is_banned = ")
			.push_bind(is_banned);

		filter.switch();
	}

	super::push_limit(&mut query, offset, limit);

	let players = query
		.build_query_as::<database::PlayerWithPlaytime>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.map(Into::into)
		.collect::<Vec<res::Player>>();

	if players.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(players))
}

#[tracing::instrument(skip(state))]
#[utoipa::path(get, tag = "Players", context_path = "/api", path = "/players/{ident}",
	params(("ident" = PlayerIdentifier, Path, description = "The player's `SteamID` or name")),
	responses(
		responses::Ok<res::Player>,
		responses::NoContent,
		responses::BadRequest,
		responses::InternalServerError,
	),
)]
pub async fn get_player(
	state: State,
	Path(ident): Path<PlayerIdentifier<'_>>,
) -> Result<Json<res::Player>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		PlayerIdentifier::SteamID(steam_id) => {
			query
				.push(" p.steam_id = ")
				.push_bind(steam_id.as_u32());
		}
		PlayerIdentifier::Name(name) => {
			query
				.push(" p.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let player = query
		.build_query_as::<database::PlayerWithPlaytime>()
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::NoContent)?
		.into();

	Ok(Json(player))
}

/// Information about a new KZ player.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's `SteamID`.
	steam_id: SteamID,

	/// The player's Steam name.
	name: String,

	/// The player's IP address.
	#[schema(value_type = String)]
	ip_address: Ipv4Addr,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(post, tag = "Players", context_path = "/api", path = "/players",
	request_body = NewPlayer,
	responses(
		responses::Created<()>,
		responses::BadRequest,
		responses::Unauthorized,
		responses::InternalServerError,
	),
)]
pub async fn create_player(
	state: State,
	Json(NewPlayer { steam_id, name, ip_address }): Json<NewPlayer>,
) -> Result<Created<()>> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (steam_id, name, last_known_ip_address)
		VALUES
			(?, ?, ?)
		"#,
		steam_id.as_u32(),
		name,
		ip_address.to_string(),
	}
	.execute(state.database())
	.await?;

	Ok(Created(()))
}

/// Updated information about a KZ player.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's new name.
	name: Option<String>,

	/// The player's new IP address.
	#[schema(value_type = String)]
	ip_address: Option<Ipv4Addr>,

	session_data: SessionData,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SessionData {
	/// Amount of seconds spent with a running timer.
	time_active: u32,

	/// Amount of seconds spent spectating.
	time_spectating: u32,

	/// Amount of seconds spent inactive.
	time_afk: u32,

	/// How many perfect bhops the player has hit in total.
	perfs: u16,

	/// How many bhops the player has hit 0 ticks after landing.
	bhops_tick0: u16,

	/// How many bhops the player has hit 1 ticks after landing.
	bhops_tick1: u16,

	/// How many bhops the player has hit 2 ticks after landing.
	bhops_tick2: u16,

	/// How many bhops the player has hit 3 ticks after landing.
	bhops_tick3: u16,

	/// How many bhops the player has hit 4 ticks after landing.
	bhops_tick4: u16,

	/// How many bhops the player has hit 5 ticks after landing.
	bhops_tick5: u16,

	/// How many bhops the player has hit 6 ticks after landing.
	bhops_tick6: u16,

	/// How many bhops the player has hit 7 ticks after landing.
	bhops_tick7: u16,

	/// How many bhops the player has hit 8 ticks after landing.
	bhops_tick8: u16,
}

#[tracing::instrument(skip(state))]
#[utoipa::path(put, tag = "Players", context_path = "/api", path = "/players/{steam_id}",
	params(("steam_id" = SteamID, Path, description = "The player's SteamID")),
	request_body = PlayerUpdate,
	responses(
		responses::Ok<()>,
		responses::BadRequest,
		responses::Unauthorized,
		responses::InternalServerError,
	),
)]
pub async fn update_player(
	state: State,
	Extension(server): Extension<AuthenticatedServer>,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate { name, ip_address, session_data }): Json<PlayerUpdate>,
) -> Result<()> {
	let steam32_id = steam_id.as_u32();
	let mut transaction = state.transaction().await?;

	if let Some(name) = name {
		sqlx::query!("UPDATE Players SET name = ? WHERE steam_id = ?", name, steam32_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(ip_address) = ip_address.map(|ip| ip.to_string()) {
		sqlx::query! {
			r#"
			UPDATE
				Players
			SET
				last_known_ip_address = ?
			WHERE
				steam_id = ?
			"#,
			ip_address,
			steam32_id
		}
		.execute(transaction.as_mut())
		.await?;
	}

	sqlx::query! {
		r#"
		INSERT INTO
			Sessions (
				player_id,
				server_id,
				time_active,
				time_spectating,
				time_afk,
				perfs,
				bhops_tick0,
				bhops_tick1,
				bhops_tick2,
				bhops_tick3,
				bhops_tick4,
				bhops_tick5,
				bhops_tick6,
				bhops_tick7,
				bhops_tick8
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam32_id,
		server.id,
		session_data.time_active,
		session_data.time_spectating,
		session_data.time_afk,
		session_data.perfs,
		session_data.bhops_tick0,
		session_data.bhops_tick1,
		session_data.bhops_tick2,
		session_data.bhops_tick3,
		session_data.bhops_tick4,
		session_data.bhops_tick5,
		session_data.bhops_tick6,
		session_data.bhops_tick7,
		session_data.bhops_tick8,
	}
	.execute(transaction.as_mut())
	.await?;

	transaction.commit().await?;

	Ok(())
}
