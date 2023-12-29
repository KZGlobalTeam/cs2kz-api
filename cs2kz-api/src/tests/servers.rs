use std::net::Ipv4Addr;

use cs2kz::SteamID;

use crate::models::{Player, Server};

#[crate::test]
async fn get(ctx: Context) {
	let all_servers = ctx
		.client
		.get(ctx.url("/servers"))
		.send()
		.await?
		.json::<Vec<Server>>()
		.await?;

	assert_eq!(all_servers.len(), 1, "incorrect amount of servers: {all_servers:#?}");

	let alphas_kz = ctx
		.client
		.get(ctx.url("/servers/alpha"))
		.send()
		.await?
		.json::<Server>()
		.await?;

	assert_eq!(alphas_kz.id, 1);
	assert_eq!(alphas_kz.name, "Alpha's KZ");
	assert_eq!(alphas_kz.ip_address.ip(), &Ipv4Addr::new(127, 0, 0, 1));
	assert_eq!(alphas_kz.ip_address.port(), 27015);
	assert_eq!(alphas_kz.owned_by, Player {
		steam_id: SteamID::from_u32(322356345)?,
		name: String::from("AlphaKeks"),
	});
}
