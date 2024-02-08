use std::net::Ipv4Addr;

use color_eyre::eyre::ensure;
use cs2kz::SteamID;

use crate::players::Player;
use crate::servers::Server;

#[crate::test]
async fn get(ctx: Context) {
	let all_servers = ctx
		.http_client
		.get(ctx.url("/servers"))
		.send()
		.await?
		.json::<Vec<Server>>()
		.await?;

	#[rustfmt::skip]
	ensure!(all_servers.len() == 1, "incorrect amount of servers: {all_servers:#?}");

	let alphas_kz = ctx
		.http_client
		.get(ctx.url("/servers/alpha"))
		.send()
		.await?
		.json::<Server>()
		.await?;

	ensure!(alphas_kz.id == 1);
	ensure!(alphas_kz.name == "Alpha's KZ");
	ensure!(alphas_kz.ip_address.ip() == &Ipv4Addr::new(127, 0, 0, 1));
	ensure!(alphas_kz.ip_address.port() == 27015);

	#[rustfmt::skip]
	ensure!(alphas_kz.owned_by == Player {
		steam_id: SteamID::from_u32(322356345)?,
		name: String::from("AlphaKeks"),
	});
}
