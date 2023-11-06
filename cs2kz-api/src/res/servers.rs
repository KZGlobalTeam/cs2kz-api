use {
	super::PlayerInfo,
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::{mysql::MySqlRow, FromRow, Row},
	std::net::Ipv4Addr,
	utoipa::ToSchema,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Server {
	pub id: u16,
	pub name: String,
	pub owned_by: PlayerInfo,

	#[schema(value_type = String)]
	pub ip: Ipv4Addr,

	pub port: u16,
}

// `Ipv4Addr` does not implement `TryFrom<String>`, only `FromStr`.
// This means that we can't use a derive implementation (with e.g. `#[sqlx(try_from = "String")]`),
// but instead have to implement `FromRow` manually.
impl FromRow<'_, MySqlRow> for Server {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		let id = row.try_get("id")?;
		let name = row.try_get("name")?;
		let player_name = row.try_get("player_name")?;
		let steam32_id = row.try_get("steam_id")?;
		let port = row.try_get("port")?;

		let steam_id =
			SteamID::from_id32(steam32_id).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let ip = row
			.try_get::<&str, _>("ip_address")?
			.parse::<Ipv4Addr>()
			.map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

		let owned_by = PlayerInfo { name: player_name, steam_id };

		Ok(Self { id, name, owned_by, ip, port })
	}
}
