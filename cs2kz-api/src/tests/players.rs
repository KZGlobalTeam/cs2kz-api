use color_eyre::Result;
use sqlx::MySqlPool;

use super::Context;
use crate::models::Player;

#[sqlx::test(
	migrator = "super::MIGRATOR",
	fixtures(path = "../../../database/fixtures", scripts("players.sql"))
)]
async fn get(pool: MySqlPool) -> Result<()> {
	let cx = Context::new(pool).await?;

	let all_players = cx
		.client
		.get(cx.url("/players"))
		.send()
		.await?
		.json::<Vec<Player>>()
		.await?;

	assert_eq!(all_players.len(), 8, "incorrect amount of players");

	let has_ibrahizy = all_players
		.iter()
		.any(|player| player.steam_id.as_u32() == 304674089);

	assert!(has_ibrahizy, "missing iBrahizy");

	let ibrahizy = cx
		.client
		.get(cx.url("/players/304674089"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(ibrahizy.steam_id.as_u32(), 304674089);
	assert_eq!(ibrahizy.name, "iBrahizy");

	let alphakeks = cx
		.client
		.get(cx.url("/players/STEAM_1:1:161178172"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(alphakeks.steam_id.as_u32(), 322356345);
	assert_eq!(alphakeks.name, "AlphaKeks");

	let zer0k = cx
		.client
		.get(cx.url("/players/er0."))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(zer0k.steam_id.as_u32(), 158416176);
	assert_eq!(zer0k.name, "zer0.k");

	Ok(())
}
