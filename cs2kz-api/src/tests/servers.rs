use std::net::Ipv4Addr;

use color_eyre::Result;
use cs2kz::SteamID;
use sqlx::MySqlPool;

use super::Context;
use crate::models::{Player, Server};

#[sqlx::test(
	migrator = "super::MIGRATOR",
	fixtures(
		path = "../../../database/fixtures",
		scripts("players.sql", "servers.sql"),
	)
)]
async fn get(pool: MySqlPool) -> Result<()> {
	let cx = Context::new(pool).await?;

	let all_servers = cx
		.client
		.get(cx.url("/servers"))
		.send()
		.await?
		.json::<Vec<Server>>()
		.await?;

	assert_eq!(all_servers.len(), 1, "incorrect amount of servers: {all_servers:#?}");

	let alphas_kz = cx
		.client
		.get(cx.url("/servers/alpha"))
		.send()
		.await?
		.json::<Server>()
		.await?;

	assert_eq!(alphas_kz.id, 1);
	assert_eq!(alphas_kz.name, "Alpha's KZ");
	assert_eq!(alphas_kz.ip_address.ip(), &Ipv4Addr::new(127, 0, 0, 1));
	assert_eq!(alphas_kz.ip_address.port(), 1337);
	assert_eq!(alphas_kz.owned_by, Player {
		steam_id: SteamID::from_u32(322356345)?,
		name: String::from("AlphaKeks"),
	});

	Ok(())
}
