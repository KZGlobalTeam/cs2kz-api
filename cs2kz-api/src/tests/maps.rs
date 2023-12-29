use color_eyre::eyre::ContextCompat;

use crate::models::{KZMap, Player};

#[crate::test]
async fn get(ctx: Context) {
	let all_maps = ctx
		.client
		.get(ctx.url("/maps"))
		.send()
		.await?
		.json::<Vec<KZMap>>()
		.await?;

	assert_eq!(all_maps.len(), 8, "incorrect amount of maps: {all_maps:#?}");

	let victoria = ctx
		.client
		.get(ctx.url("/maps/victoria"))
		.send()
		.await?
		.json::<KZMap>()
		.await?;

	assert_eq!(victoria.id, 5);
	assert_eq!(victoria.name, "kz_victoria");
	assert_eq!(victoria.courses.len(), 2);

	let stage_2 = victoria
		.courses
		.iter()
		.find(|c| c.stage == 2)
		.context("missing stage 2 on kz_victoria")?;

	assert_eq!(stage_2.mappers, vec![Player {
		steam_id: "STEAM_1:1:207612938".parse()?,
		name: String::from("lars"),
	}]);
}
