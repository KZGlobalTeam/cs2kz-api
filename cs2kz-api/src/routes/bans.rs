use {
	crate::{
		res::{bans as res, BadRequest},
		util::{Created, Filter},
		Error, Response, Result, State,
	},
	axum::{extract::Query, Json},
	chrono::{DateTime, Utc},
	cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

#[derive(Debug, Deserialize, IntoParams)]
pub struct BanGetParams<'a> {
	player: Option<PlayerIdentifier<'a>>,
	reason: Option<String>,
	server: Option<ServerIdentifier<'a>>,
	expired: Option<bool>,
	created_after: Option<DateTime<Utc>>,
	created_before: Option<DateTime<Utc>>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Bans", context_path = "/api/v0", path = "/bans", params(BanGetParams), responses(
	(status = 200, body = Vec<Ban>),
	(status = 204),
	(status = 400, response = BadRequest),
	(status = 500, body = Error),
))]
pub async fn get_bans(
	state: State,
	Query(BanGetParams { player, reason, server, expired, created_after, created_before }): Query<
		BanGetParams<'_>,
	>,
) -> Response<Vec<res::Ban>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
			p.id,
			p.name,
			b.reason,
			b.created_on
		FROM
			Players p
			JOIN Bans b ON b.player_id = p.id
		"#,
	);

	let mut filter = Filter::new();

	if let Some(player) = player {
		query.push(filter);

		match player {
			PlayerIdentifier::SteamID(steam_id) => {
				query
					.push(" p.id = ")
					.push_bind(steam_id.as_u32());
			}
			PlayerIdentifier::Name(name) => {
				query
					.push(" p.name LIKE ")
					.push_bind(format!("%{name}%"));
			}
		};

		filter.switch();
	}

	if let Some(ref reason) = reason {
		query
			.push(filter)
			.push(" b.reason = ")
			.push_bind(reason);

		filter.switch();
	}

	if let Some(server) = server {
		let server_id = match server {
			ServerIdentifier::ID(id) => id,
			ServerIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Servers WHERE name = ?", name)
					.fetch_one(state.database())
					.await?
					.id
			}
		};

		query
			.push(filter)
			.push(" b.server_id = ")
			.push_bind(server_id);

		filter.switch();
	}

	if let Some(expired) = expired {
		let now = Utc::now();

		query
			.push(filter)
			.push(" b.expires_on ")
			.push(if expired { " < " } else { " > " })
			.push_bind(now);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" b.created_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" b.created_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	let bans = query
		.build_query_as::<res::Ban>()
		.fetch_all(state.database())
		.await?;

	if bans.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(bans))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewBan {
	steam_id: SteamID,
	ip: Ipv4Addr,
	server_id: Option<u16>,
	reason: String,
	banned_by: Option<SteamID>,
	plugin_version: u16,
	expires_on: Option<DateTime<Utc>>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Bans", context_path = "/api/v0", path = "/bans", request_body = NewBan, responses(
	(status = 201, body = NewBan),
	(status = 400, response = BadRequest),
	(status = 401, body = Error),
	(status = 500, body = Error),
))]
pub async fn create_ban(
	state: State,
	Query(NewBan { steam_id, ip, server_id, reason, banned_by, plugin_version, expires_on }): Query<
		NewBan,
	>,
) -> Result<Created<Json<NewBan>>> {
	sqlx::query! {
		r#"
		INSERT INTO
			Bans (
				player_id,
				player_ip,
				server_id,
				reason,
				banned_by,
				plugin_version,
				expires_on
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id.as_u32(),
		ip.to_string(),
		server_id,
		reason,
		banned_by.map(|steam_id| steam_id.as_u32()),
		plugin_version,
		expires_on,
	}
	.execute(state.database())
	.await?;

	Ok(Created(Json(NewBan {
		steam_id,
		ip,
		server_id,
		reason,
		banned_by,
		plugin_version,
		expires_on,
	})))
}
