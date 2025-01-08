use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let schema = cs2kz_api::openapi::schema()
        .to_pretty_json()
        .context("failed to serialize OpenAPI schema")?;

    print!("{schema}");

    Ok(())
}
