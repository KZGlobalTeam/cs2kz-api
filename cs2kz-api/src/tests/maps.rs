use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use cs2kz::SteamID;
use sqlx::MySqlPool;

use super::Context;
use crate::models::{KZMap, Player};

#[sqlx::test(
	migrator = "super::MIGRATOR",
	fixtures(
		path = "../../../database/fixtures",
		scripts("players.sql", "maps.sql"),
	)
)]
async fn get(pool: MySqlPool) -> Result<()> {
	let cx = Context::new(pool).await?;

	let all_maps = cx
		.client
		.get(cx.url("/maps"))
		.send()
		.await?
		.json::<Vec<KZMap>>()
		.await?;

	assert_eq!(all_maps.len(), 2, "incorrect amount of maps: {all_maps:#?}");

	let victoria = cx
		.client
		.get(cx.url("/maps/victoria"))
		.send()
		.await?
		.json::<KZMap>()
		.await?;

	assert_eq!(victoria.id, 2);
	assert_eq!(victoria.name, "kz_victoria");
	assert_eq!(victoria.courses.len(), 2);

	let stage_2 = victoria
		.courses
		.iter()
		.find(|c| c.stage == 2)
		.context("missing stage 2 on kz_victoria")?;

	assert_eq!(stage_2.mappers, vec![Player {
		steam_id: SteamID::from_u32(117087881)?,
		name: String::from("Kiwi"),
	}]);

	Ok(())
}
