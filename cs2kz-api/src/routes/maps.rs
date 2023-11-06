use {
	crate::{
		res::{maps as res, BadRequest},
		util::{self, Created},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{MapIdentifier, PlayerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	utoipa::{IntoParams, ToSchema},
};

const LIMIT_DEFAULT: u64 = 100;
const LIMIT_MAX: u64 = 1000;

// FIXME(AlphaKeks): this does not include the courses yet
const ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		m.id,
		m.name,
		m.workshop_id,
		m.filesize,
		p1.name player_name,
		p1.id steam_id,
		m.created_on,
		c.id course_id,
		c.stage course_stage,
		c.difficulty course_tier,
		p2.id course_created_by_id,
		p2.name course_created_by_name
	FROM
		Maps m
		JOIN Players p1 ON p1.id = m.owned_by
		JOIN Courses c ON c.map_id = m.id
		JOIN Players p2 ON p2.id = c.created_by
"#;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetMapsParams<'a> {
	pub name: Option<String>,
	pub created_by: Option<PlayerIdentifier<'a>>,
	pub created_after: Option<DateTime<Utc>>,
	pub created_before: Option<DateTime<Utc>>,

	#[serde(default)]
	pub offset: u64,
	pub limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps", params(GetMapsParams), responses(
	(status = 200, body = Vec<KZMap>),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_maps(
	state: State,
	Query(GetMapsParams { name, created_by, created_after, created_before, offset, limit }): Query<
		GetMapsParams<'_>,
	>,
) -> Response<Vec<res::KZMap>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = util::Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" m.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(player) = created_by {
		let steam32_id = match player {
			PlayerIdentifier::SteamID(steam_id) => steam_id.as_u32(),
			PlayerIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Players WHERE name LIKE ?", name)
					.fetch_one(state.database())
					.await?
					.id
			}
		};

		query
			.push(filter)
			.push(" p1.id = ")
			.push_bind(steam32_id);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" m.created_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" m.created_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	query.push(" GROUP BY m.id ");

	let limit = limit.map_or(LIMIT_DEFAULT, |limit| std::cmp::min(limit, LIMIT_MAX));

	query
		.push(" LIMIT ")
		.push_bind(offset)
		.push(",")
		.push_bind(limit);

	let maps = query
		.build_query_as::<res::KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.fold(Vec::<res::KZMap>::new(), |mut maps, mut map| {
			if let Some(last_map) = maps.last_mut() {
				if last_map.id == map.id {
					last_map.courses.append(&mut map.courses);
					return maps;
				}
			};

			maps.push(map);
			maps
		});

	if maps.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(maps))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Maps", context_path = "/api/v0", path = "/maps/{ident}", params(
	("ident" = MapIdentifier, Path, description = "The map's ID or name")
), responses(
	(status = 200, body = KZMap),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_map(state: State, Path(ident): Path<MapIdentifier<'_>>) -> Response<res::KZMap> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		MapIdentifier::ID(id) => {
			query.push(" m.id = ").push_bind(id);
		}
		MapIdentifier::Name(name) => {
			query
				.push(" m.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let map = query
		.build_query_as::<res::KZMap>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.reduce(|mut acc, mut row| {
			acc.courses.append(&mut row.courses);
			acc
		})
		.ok_or(Error::NoContent)?;

	Ok(Json(map))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMap {
	name: String,
	workshop_id: u32,
	filters: Vec<Filter>,
	created_by: SteamID,
	approved_by: SteamID,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Filter {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewMapWithId {
	id: u16,

	#[serde(flatten)]
	map: NewMap,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Maps", context_path = "/api/v0", path = "/maps", request_body = NewMap, responses(
	(status = 201, body = NewMapWithId),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn create_map(
	state: State,
	Json(NewMap { name, workshop_id, filters, created_by, approved_by }): Json<NewMap>,
) -> Result<Created<Json<NewMapWithId>>> {
	todo!();
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MapUpdate {
	name: Option<String>,
	workshop_id: Option<u32>,
	filters_added: Option<Vec<Filter>>,
	filters_removed: Option<Vec<u16>>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Maps", context_path = "/api/v0", path = "/maps/{id}", request_body = MapUpdate, params(
	("id" = u16, Path, description = "The map's ID")
), responses(
	(status = 200),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn update_map(
	state: State,
	Path(map_id): Path<u16>,
	Json(MapUpdate { name, workshop_id, filters_added, filters_removed }): Json<MapUpdate>,
) -> Response<()> {
	todo!();
}
