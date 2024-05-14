use std::path::PathBuf;
use std::{fs, process};

use anyhow::Context;
use clap::Parser;
use cs2kz_api::API;
use similar::TextDiff;

#[derive(Parser)]
struct Args {
	/// Diff the generated spec against an existing one.
	///
	/// If there is any diff at all, it will be emitted on stderr and the program will exit
	/// with code 1.
	#[arg(long, name = "FILE")]
	check: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
	let args = Args::parse();
	let spec = API::spec();

	let Some(path) = args.check else {
		print!("{spec}");
		return Ok(());
	};

	let file = fs::read_to_string(&path).with_context(|| format!("read {path:?}"))?;
	let diff = TextDiff::from_lines(&file, &spec);
	let mut has_diff = false;

	for hunk in diff.unified_diff().iter_hunks() {
		eprintln!("{hunk}");
		has_diff = true;
	}

	if has_diff {
		process::exit(1);
	}

	Ok(())
}
