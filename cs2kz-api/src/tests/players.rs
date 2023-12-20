use crate::models::Player;

#[crate::test("players.sql")]
async fn get(ctx: Context) {
	let all_players = ctx
		.client
		.get(ctx.url("/players"))
		.send()
		.await?
		.json::<Vec<Player>>()
		.await?;

	assert_eq!(all_players.len(), 8, "incorrect amount of players");

	let has_ibrahizy = all_players
		.iter()
		.any(|player| player.steam_id.as_u32() == 304674089);

	assert!(has_ibrahizy, "missing iBrahizy");

	let ibrahizy = ctx
		.client
		.get(ctx.url("/players/304674089"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(ibrahizy.steam_id.as_u32(), 304674089);
	assert_eq!(ibrahizy.name, "iBrahizy");

	let alphakeks = ctx
		.client
		.get(ctx.url("/players/STEAM_1:1:161178172"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(alphakeks.steam_id.as_u32(), 322356345);
	assert_eq!(alphakeks.name, "AlphaKeks");

	let zer0k = ctx
		.client
		.get(ctx.url("/players/er0."))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(zer0k.steam_id.as_u32(), 158416176);
	assert_eq!(zer0k.name, "zer0.k");

	Ok(())
}
