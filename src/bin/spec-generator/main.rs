use std::path::PathBuf;
use std::{fs, process};

use clap::Parser;
use color_eyre::eyre::Context;
use color_eyre::Result;
use cs2kz_api::API;
use similar::TextDiff;

/// This tool will generate a JSON representation of the API's OpenAPI specification.
/// Running it without any arguments will simply emit the spec to stdout.
#[derive(Parser)]
struct Args {
	/// Output the spec into a file intead of stdout.
	#[arg(short, long)]
	output: Option<PathBuf>,

	/// Diff the spec against the file at the specified path.
	///
	/// If this produces any diff, the diff will be printed to stderr and the program will exit
	/// with a non-0 exit code.
	#[arg(long, name = "PATH")]
	check: Option<PathBuf>,
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = Args::parse();
	let spec = API::spec();

	if let Some(path) = args.check {
		let other =
			fs::read_to_string(path).context("Failed to read other file to diff against.")?;

		let diff = TextDiff::from_lines(&other, &spec);
		let mut has_diff = false;

		for hunk in diff.unified_diff().iter_hunks() {
			eprintln!("{hunk}");
			has_diff = true;
		}

		if has_diff {
			eprintln!("Diffs in the spec were found. Please update it using `make api-spec`.");
			process::exit(1);
		}

		return Ok(());
	}

	let Some(output) = args.output else {
		println!("{spec}");
		return Ok(());
	};

	fs::write(&output, spec.into_bytes())
		.with_context(|| format!("Failed to write spec to `{}`.", output.display()))?;

	Ok(())
}
