use std::fs::File;
use std::path::PathBuf;
use std::time::Instant;
use std::{io, iter};

use clap::Parser;
use cs2kz_api_points::Record;
use eyre::{Context, Result};
use pyo3::Python;

#[derive(Parser)]
struct Args {
	#[arg(short, long)]
	output: Option<PathBuf>,
	file: Option<PathBuf>,
}

fn main() -> Result<()> {
	let args = Args::parse();
	let (mut file, mut stdin);
	let reader: &mut dyn io::Read = match args.file.as_deref() {
		Some(path) => {
			file = File::open(path).with_context(|| format!("open {path:?}"))?;
			&mut file
		}
		None => {
			stdin = io::stdin().lock();
			&mut stdin
		}
	};

	let start = Instant::now();

	eprintln!("[{:?}] parsing input...", start.elapsed());

	let mut records = serde_json::from_reader::<_, Vec<Record>>(reader).context("parse records")?;

	eprintln!("[{:?}] sorting input...", start.elapsed());

	records.sort_unstable();

	let results = Python::with_gil(|py| {
		eprintln!("[{:?}] calculating points...", start.elapsed());
		cs2kz_api_points::calculate_points(py, &records)
	})
	.map(|times| iter::zip(&records, times))
	.context("calculate points")?;

	eprintln!("[{:?}] writing output...", start.elapsed());

	if let Some(path) = args.output.as_deref() {
		let file = File::options()
			.write(true)
			.create(true)
			.truncate(true)
			.open(path)
			.with_context(|| format!("open {path:?}"))?;

		print_results(results, file)?;
	} else {
		print_results(results, io::stdout().lock())?;
	}

	eprintln!("[{:?}] done", start.elapsed());

	Ok(())
}

fn print_results<'r>(
	results: impl IntoIterator<Item = (&'r Record, u16)>,
	mut writer: impl io::Write,
) -> Result<()> {
	writeln!(&mut writer, "| Rank | Player | SteamID | Time | Points |")?;
	writeln!(&mut writer, "|------|--------|---------|------|--------|")?;

	for (rank, (record, points)) in results.into_iter().enumerate() {
		let rank = rank + 1;
		let player = &record.player_name;
		let steam_id = record.steam_id;
		let time = record.time;

		writeln!(&mut writer, "| {rank} | {player} | {steam_id} | {time} | {points} |")?;
	}

	Ok(())
}
