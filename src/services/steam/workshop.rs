//! This module contains types and functions for working with Steam's workshop.
//!
//! Namely, [`WorkshopID`] and [`MapFile`], which can be used for downloading
//! maps from the workshop using [DepotDownloader].
//!
//! [DepotDownloader]: https://github.com/SteamRE/DepotDownloader

use std::io;
use std::path::Path;

use tap::{Pipe, TryConv};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

crate::macros::make_id! {
	/// An ID uniquely identifying a Steam Workshop item.
	WorkshopID as u32
}

/// A handle to a downloaded workshop map.
#[derive(Debug)]
#[must_use = "`MapFile` contains a file handle"]
pub struct MapFile
{
	/// OS handle to the open file descriptor.
	handle: File,
}

impl MapFile
{
	/// Downloads a map using [DepotDownloader] and returns a handle to the
	/// `.vpk` file.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[tracing::instrument(level = "trace", err(Debug, level = "debug"))]
	pub(super) async fn download(
		id: WorkshopID,
		artifacts_path: &Path,
		depot_downloader_path: &Path,
	) -> io::Result<Self>
	{
		let out_dir = artifacts_path;
		let depot_downloader = depot_downloader_path;

		tracing::debug!(?out_dir, "invoking {depot_downloader:?}");

		let result = Command::new(depot_downloader)
			.args(["-app", "730", "-pubfile"])
			.arg(id.to_string())
			.arg("-dir")
			.arg(out_dir)
			.spawn()?
			.wait_with_output()
			.await?;

		let mut stdout = tokio::io::stdout();
		let mut stderr = tokio::io::stderr();

		if let Err(error) = tokio::try_join!(stdout.flush(), stderr.flush()) {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				%error,
				"failed to flush stdout/stderr",
			};
		}

		if !result.status.success() {
			return Err(io::Error::other("DepotDownloader did not complete successfully"));
		}

		let out_file_path = out_dir.join(format!("{id}.vpk"));
		let handle = File::open(&out_file_path).await.inspect_err(|err| {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				%err,
				path = ?out_file_path,
				"failed to open map file",
			};
		})?;

		Ok(Self { handle })
	}

	/// Computes the MD5 checksum of this file.
	#[tracing::instrument(
		level = "trace",
		skip(self),
		ret(level = "debug"),
		err(Debug, level = "debug")
	)]
	pub async fn checksum(mut self) -> io::Result<md5::Digest>
	{
		let mut buf = self
			.handle
			.metadata()
			.await?
			.pipe(|metadata| metadata.len())
			.try_conv::<usize>()
			.map(Vec::with_capacity)
			.expect("64-bit platform");

		self.handle.read_to_end(&mut buf).await?;

		Ok(md5::compute(&buf))
	}
}
