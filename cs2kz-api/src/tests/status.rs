#[crate::test]
async fn basic(ctx: Context) -> Result<()> {
	let schnose = ctx.client.get(ctx.url("/")).send().await?.text().await?;

	assert_eq!(schnose, "(͡ ͡° ͜ つ ͡͡°)");

	Ok(())
}
