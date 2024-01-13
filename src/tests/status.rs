use color_eyre::eyre::ensure;

#[crate::test]
async fn hello_world(ctx: Context) {
	let url = ctx.url("/");
	let schnose = ctx.http_client.get(url).send().await?.text().await?;

	ensure!(schnose == "(͡ ͡° ͜ つ ͡͡°)");
}
