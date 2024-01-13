use color_eyre::eyre::{ensure, ContextCompat};

use crate::maps::KZMap;
use crate::players::Player;

#[crate::test]
async fn get(ctx: Context) {
	let all_maps = ctx
		.http_client
		.get(ctx.url("/maps"))
		.send()
		.await?
		.json::<Vec<KZMap>>()
		.await?;

	ensure!(all_maps.len() == 8, "incorrect amount of maps: {all_maps:#?}");

	let victoria = ctx
		.http_client
		.get(ctx.url("/maps/victoria"))
		.send()
		.await?
		.json::<KZMap>()
		.await?;

	ensure!(victoria.id == 5);
	ensure!(victoria.name == "kz_victoria");
	ensure!(victoria.courses.len() == 2);

	let stage_2 = victoria
		.courses
		.iter()
		.find(|c| c.stage == 2)
		.context("missing stage 2 on kz_victoria")?;

	ensure!(
		stage_2.mappers
			== vec![Player {
				steam_id: "STEAM_1:1:207612938".parse()?,
				name: String::from("lars"),
				is_banned: false,
			}]
	);
}

#[crate::test]
async fn get_by_mapper(ctx: Context) {
	let maps = ctx
		.http_client
		.get(ctx.url("/maps"))
		.query(&[("mapper", "gamechaos")])
		.send()
		.await?
		.json::<Vec<KZMap>>()
		.await?;

	ensure!(maps.len() >= 3);

	let victoria = maps
		.iter()
		.find(|map| map.id == 5)
		.context("missing kz_victoria")?;

	ensure!(victoria.mappers.len() == 3);

	let has_mapper = |steam_id: u64| {
		victoria
			.mappers
			.iter()
			.any(|mapper| mapper.steam_id.as_u64() == steam_id)
	};

	ensure!(has_mapper(76561198165203332), "missing gamechaos");
	ensure!(has_mapper(76561198045869085), "missing mark");
	ensure!(has_mapper(76561198375491605), "missing lars");
}
