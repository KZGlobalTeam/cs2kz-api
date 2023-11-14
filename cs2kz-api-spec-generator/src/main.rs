use {
	clap::{Parser, Subcommand},
	color_eyre::{eyre::Context, Result},
	cs2kz_api::API,
	similar::TextDiff,
	std::path::PathBuf,
};

#[derive(Parser)]
struct Args {
	#[command(subcommand)]
	output: Output,
}

#[derive(Subcommand)]
enum Output {
	/// Do not write the output anywhere, just check against the existing output.
	Check {
		/// The path to the existing spec file.
		#[clap(default_value = "./api-spec.json")]
		path: PathBuf,
	},

	/// Write the generated spec to STDOUT.
	Stdout,

	/// Write the generated spec to a JSON file.
	Json {
		/// The path to the target spec file.
		#[clap(default_value = "./api-spec.json")]
		path: PathBuf,
	},
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let json = API::json()?;

	match args.output {
		Output::Check { path } => {
			let check = std::fs::read_to_string(path).context("Failed to read spec file.")?;
			let diff = TextDiff::from_lines(&check, &json);
			let mut error = false;

			for hunk in diff.unified_diff().iter_hunks() {
				error = true;
				eprintln!("{hunk}");
			}

			if error {
				std::process::exit(1);
			}
		}
		Output::Stdout => println!("{json}"),
		Output::Json { path } => {
			std::fs::write(path, json.into_bytes()).context("Failed to write JSON to disk.")?;
		}
	};

	Ok(())
}
