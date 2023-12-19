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

	assert_eq!(all_players.len(), 5, "incorrect amount of players");

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

	let ibrahizy = cx
		.client
		.get(cx.url("/players/STEAM_0:1:152337044"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(ibrahizy.steam_id.as_u32(), 304674089);
	assert_eq!(ibrahizy.name, "iBrahizy");

	let ibrahizy = cx
		.client
		.get(cx.url("/players/brahi"))
		.send()
		.await?
		.json::<Player>()
		.await?;

	assert_eq!(ibrahizy.steam_id.as_u32(), 304674089);
	assert_eq!(ibrahizy.name, "iBrahizy");

	Ok(())
}
