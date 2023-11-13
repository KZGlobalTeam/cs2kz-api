use {
	clap::{Parser, Subcommand},
	color_eyre::{eyre::Context, Result},
	cs2kz_api::API,
	std::path::PathBuf,
};

#[derive(Parser)]
struct Args {
	#[command(subcommand)]
	output: Output,
}

#[derive(Subcommand)]
enum Output {
	Stdout,
	Json { path: PathBuf },
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let json = API::json()?;

	match args.output {
		Output::Stdout => println!("{json}"),
		Output::Json { path } => {
			std::fs::write(path, json.into_bytes()).context("Failed to write JSON to disk.")?;
		}
	};

	Ok(())
}
