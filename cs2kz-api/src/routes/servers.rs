use {
	crate::{
		res::{servers as res, BadRequest},
		util::{Created, Filter},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

const LIMIT_DEFAULT: u64 = 100;
const LIMIT_MAX: u64 = 500;

const ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		s.id,
		s.name,
		p.name player_name,
		p.id steam_id,
		s.ip_address,
		s.port
	FROM
		Servers s
		JOIN Players p ON p.id = s.owned_by
"#;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetServersParams<'a> {
	name: Option<String>,
	owned_by: Option<PlayerIdentifier<'a>>,
	created_after: Option<DateTime<Utc>>,
	created_before: Option<DateTime<Utc>>,

	#[serde(default)]
	offset: u64,
	limit: Option<u64>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers",
	params(GetServersParams),
	responses(
		(status = 200, body = Vec<Server>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_servers(
	state: State,
	Query(GetServersParams { name, owned_by, created_after, created_before, offset, limit }): Query<
		GetServersParams<'_>,
	>,
) -> Response<Vec<res::Server>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" s.name LIKE ")
			.push_bind(name);

		filter.switch();
	}

	if let Some(player) = owned_by {
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
			.push(" p.id = ")
			.push_bind(steam32_id);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" s.approved_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" s.approved_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	let limit = limit.map_or(LIMIT_DEFAULT, |limit| std::cmp::min(limit, LIMIT_MAX));

	query
		.push(" LIMIT ")
		.push_bind(offset)
		.push(",")
		.push_bind(limit);

	let servers = query
		.build_query_as::<res::Server>()
		.fetch_all(state.database())
		.await?;

	if servers.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(servers))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers/{ident}",
	params(("ident" = ServerIdentifier, Path, description = "The servers's ID or name")),
	responses(
		(status = 200, body = Server),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_server(
	state: State,
	Path(ident): Path<ServerIdentifier<'_>>,
) -> Response<res::Server> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		ServerIdentifier::ID(id) => {
			query.push(" s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query
				.push(" s.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let server = query
		.build_query_as::<res::Server>()
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::NoContent)?;

	Ok(Json(server))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServer {
	name: String,
	owned_by: SteamID,

	#[schema(value_type = String)]
	ip: Ipv4Addr,

	port: u16,
	approved_by: SteamID,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedServer {
	id: u16,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Servers", context_path = "/api/v0", path = "/servers",
	request_body = NewServer,
	responses(
		(status = 201, body = CreatedServer),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_server(
	state: State,
	Json(NewServer { name, owned_by, ip, port, approved_by }): Json<NewServer>,
) -> Result<Created<Json<CreatedServer>>> {
	let api_key = rand::random::<u32>();
	let mut transaction = state.database().begin().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Servers (name, ip_address, port, owned_by, api_key)
		VALUES
			(?, ?, ?, ?, ?)
		"#,
		name,
		ip.to_string(),
		port,
		owned_by.as_u32(),
		api_key,
	}
	.execute(transaction.as_mut())
	.await?;

	let id = sqlx::query!("SELECT MAX(id) id FROM Servers")
		.fetch_one(transaction.as_mut())
		.await?
		.id
		.expect("server was just inserted");

	transaction.commit().await?;

	Ok(Created(Json(CreatedServer { id })))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerUpdate {
	name: String,
	owned_by: SteamID,

	#[schema(value_type = String)]
	ip: Ipv4Addr,

	port: u16,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Servers", context_path = "/api/v0", path = "/servers/{id}",
	params(("id" = u16, Path, description = "The server's ID")),
	request_body = ServerUpdate,
	responses(
		(status = 200),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn update_server(
	state: State,
	Path(server_id): Path<u16>,
	Json(ServerUpdate { name, owned_by, ip, port }): Json<ServerUpdate>,
) -> Result<()> {
	sqlx::query! {
		r#"
		UPDATE
			Servers
		SET
			name = ?,
			owned_by = ?,
			ip_address = ?,
			port = ?
		WHERE
			id = ?
		"#,
		name,
		owned_by.as_u32(),
		ip.to_string(),
		port,
		server_id,
	}
	.execute(state.database())
	.await?;

	Ok(())
}
