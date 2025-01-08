use anyhow::Context;

fn main() -> anyhow::Result<()> {
<<<<<<< HEAD
    let schema = cs2kz_api::openapi::schema()
        .to_pretty_json()
        .context("failed to serialize OpenAPI schema")?;

    print!("{schema}");

    Ok(())
=======
	let schema = cs2kz_api::openapi::schema()
		.to_pretty_json()
		.context("failed to serialize OpenAPI schema")?;

	print!("{schema}");

	Ok(())
>>>>>>> 6a8333b (workflow for OpenAPI schema)
}
