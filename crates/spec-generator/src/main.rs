use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
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

fn main() -> anyhow::Result<ExitCode> {
	let args = Args::parse();
	let spec = cs2kz_api::openapi::Spec::new().as_json();

	let Some(path) = args.check else {
		print!("{spec}");
		return Ok(ExitCode::SUCCESS);
	};

	let file = fs::read_to_string(&path).with_context(|| format!("read {path:?}"))?;
	let exit_code = TextDiff::from_lines(&file, &spec)
		.unified_diff()
		.iter_hunks()
		.fold(ExitCode::SUCCESS, |_, hunk| {
			eprintln!("{hunk}");
			ExitCode::FAILURE
		});

	Ok(exit_code)
}
