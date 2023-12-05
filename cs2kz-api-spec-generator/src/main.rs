use std::path::PathBuf;
use std::{fs, process};

use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;
use color_eyre::Result;
use cs2kz_api::API;
use similar::TextDiff;

#[derive(Parser)]
struct Args {
	#[command(subcommand)]
	output: Output,
}

#[derive(Subcommand)]
enum Output {
	/// Write the generated spec to STDOUT.
	Stdout,

	/// Write the generated spec to a JSON file.
	Json {
		/// The path to the target spec file.
		#[clap(default_value = "./api-spec.json")]
		path: PathBuf,
	},

	/// Do not write the output anywhere, just check against the existing output.
	Check {
		/// The path to the existing spec file.
		#[clap(default_value = "./api-spec.json")]
		path: PathBuf,
	},
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let json = API::json()?;

	match args.output {
		Output::Stdout => {
			println!("{json}");
		}
		Output::Json { path } => {
			fs::write(path, json.into_bytes()).context("Failed to write JSON to disk.")?;
		}
		Output::Check { path } => {
			let check = fs::read_to_string(path).context("Failed to read spec file.")?;
			let diff = TextDiff::from_lines(&check, &json);
			let mut error = false;

			for hunk in diff.unified_diff().iter_hunks() {
				error = true;
				eprintln!("{hunk}");
			}

			if error {
				process::exit(1);
			}
		}
	};

	Ok(())
}
