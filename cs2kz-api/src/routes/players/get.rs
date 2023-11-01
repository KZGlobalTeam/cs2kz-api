use {
	crate::{database, res::{player as res, BadRequest}, util::Filter, Error, Response, State},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::NaiveTime,
	cs2kz::PlayerIdentifier,
	serde::Deserialize,
	sqlx::QueryBuilder,
	utoipa::IntoParams,
};

const BASE_QUERY: &str = r#"
	SELECT
		p1.*,
		p2.playtime,
		p2.afktime
	FROM
		players AS p1
		JOIN playtimes p2 ON p2.player_id = p1.id
"#;

#[derive(Debug, Deserialize, IntoParams)]
pub struct Params {
	/// The Steam name of the player.
	name: Option<String>,

	/// The minimum amount of playtime.
	playtime: Option<NaiveTime>,

	/// Whether the player is banned.
	is_banned: Option<bool>,

	offset: Option<u64>,
	limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players", params(Params), responses(
	(status = 200, body = Vec<Player>),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn root(
	state: State,
	Query(Params { name, playtime, is_banned, offset, limit }): Query<Params>,
) -> Response<Vec<res::Player>> {
	let mut query = QueryBuilder::new(BASE_QUERY);
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

	let offset = offset.unwrap_or(0);
	let limit = limit.map_or(100, |limit| limit.min(500));

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
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players/{ident}", params(
	("ident" = PlayerIdentifier, Path, description = "The player's SteamID or name")
), responses(
	(status = 200, body = Player),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn ident(
	state: State,
	Path(ident): Path<PlayerIdentifier<'_>>,
) -> Response<res::Player> {
	let mut query = QueryBuilder::new(BASE_QUERY);

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
