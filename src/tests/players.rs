use color_eyre::eyre::ensure;

use crate::players::Player;

#[crate::test]
async fn get(ctx: Context) {
	let all_players = ctx
		.http_client
		.get(ctx.url("/players"))
		.send()
		.await?
		.json::<Vec<Player>>()
		.await?;

	ensure!(all_players.len() == 17, "incorrect amount of players");

	let has_ibrahizy = all_players
		.iter()
		.any(|player| player.steam_id.as_u32() == 304674089);

	ensure!(has_ibrahizy, "missing iBrahizy");

	let ibrahizy = ctx
		.http_client
		.get(ctx.url("/players/304674089"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	ensure!(ibrahizy.steam_id.as_u32() == 304674089);
	ensure!(ibrahizy.name == "iBrahizy");

	let alphakeks = ctx
		.http_client
		.get(ctx.url("/players/STEAM_1:1:161178172"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	ensure!(alphakeks.steam_id.as_u32() == 322356345);
	ensure!(alphakeks.name == "AlphaKeks");

	let zer0k = ctx
		.http_client
		.get(ctx.url("/players/er0."))
		.send()
		.await?
		.json::<Player>()
		.await?;

	ensure!(zer0k.steam_id.as_u32() == 158416176);
	ensure!(zer0k.name == "zer0.k");
}
