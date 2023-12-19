use color_eyre::Result;
use sqlx::MySqlPool;

use super::Context;

#[sqlx::test]
async fn basic(pool: MySqlPool) -> Result<()> {
	let cx = Context::new(pool).await?;
	let schnose = cx.client.get(cx.url("/")).send().await?.text().await?;

	assert_eq!(schnose, "(͡ ͡° ͜ つ ͡͡°)");

	Ok(())
}
