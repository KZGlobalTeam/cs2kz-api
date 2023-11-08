use {
	crate::{
		database,
		res::{player as res, BadRequest},
		util::{Created, Filter},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::NaiveTime,
	cs2kz::{PlayerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

const LIMIT_DEFAULT: u64 = 100;
const LIMIT_MAX: u64 = 500;

const ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		p1.*,
		p2.playtime,
		p2.afktime
	FROM
		players p1
		JOIN playtimes p2 ON p2.player_id = p1.id
"#;

#[derive(Debug, Deserialize, IntoParams)]
pub struct RootGetParams {
	/// The Steam name of the player.
	name: Option<String>,

	/// The minimum amount of playtime.
	playtime: Option<NaiveTime>,

	/// Whether the player is banned.
	is_banned: Option<bool>,

	#[serde(default)]
	offset: u64,
	limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players",
	params(RootGetParams),
	responses(
		(status = 200, body = Vec<Player>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_players(
	state: State,
	Query(RootGetParams { name, playtime, is_banned, offset, limit }): Query<RootGetParams>,
) -> Response<Vec<res::Player>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" p1.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(playtime) = playtime {
		query
			.push(filter)
			.push(" p1.playtime >= ")
			.push_bind(playtime);

		filter.switch();
	}

	if let Some(is_banned) = is_banned {
		query
			.push(filter)
			.push(" p1.is_banned = ")
			.push_bind(is_banned);

		filter.switch();
	}

	let limit = limit.map_or(LIMIT_DEFAULT, |limit| std::cmp::min(limit, LIMIT_MAX));

	query
		.push(" LIMIT ")
		.push_bind(offset)
		.push(",")
		.push_bind(limit);

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

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players/{ident}",
	params(("ident" = PlayerIdentifier, Path, description = "The player's SteamID or name")),
	responses(
		(status = 200, body = Player),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_player(
	state: State,
	Path(ident): Path<PlayerIdentifier<'_>>,
) -> Response<res::Player> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		PlayerIdentifier::SteamID(steam_id) => {
			query
				.push(" p1.id = ")
				.push_bind(steam_id.as_u32());
		}
		PlayerIdentifier::Name(name) => {
			query
				.push(" p1.name LIKE ")
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewPlayer {
	steam_id: SteamID,
	name: String,

	#[schema(value_type = String)]
	ip: Ipv4Addr,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Players", context_path = "/api/v0", path = "/players",
	request_body = NewPlayer,
	responses(
		(status = 201, body = NewPlayer),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_player(
	state: State,
	Json(NewPlayer { steam_id, name, ip }): Json<NewPlayer>,
) -> Result<Created<Json<NewPlayer>>> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (id, name, ip)
		VALUES
			(?, ?, ?)
		"#,
		steam_id.as_u32(),
		name,
		ip.to_string(),
	}
	.execute(state.database())
	.await?;

	Ok(Created(Json(NewPlayer { steam_id, name, ip })))
}

#[rustfmt::skip]
#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's new name.
	name: String,

	/// The player's new IP address.
	#[schema(value_type = String)]
	ip: Ipv4Addr,

	/* TODO(AlphaKeks): figure out what to take here. Probably a `course_id` as well? Maybe
	 * a `course_info` struct?
	 *
	 * /// The additional playtime recorded by the server.
	 * playtime: u32,
	 *
	 */
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Players", context_path = "/api/v0", path = "/players/{steam_id}",
	params(("steam_id" = SteamID, Path, description = "The player's SteamID")),
	request_body = PlayerUpdate,
	responses(
		(status = 200),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn update_player(
	state: State,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate { name, ip }): Json<PlayerUpdate>,
) -> Result<()> {
	// TODO(AlphaKeks): update playtimes as well
	sqlx::query! {
		r#"
		UPDATE
			Players
		SET
			name = ?,
			ip = ?
		WHERE
			id = ?
		"#,
		name,
		ip.to_string(),
		steam_id.as_u32(),
	}
	.execute(state.database())
	.await?;

	Ok(())
}
