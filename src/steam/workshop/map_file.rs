//! Workshop Map files.
//!
//! This module provides the [`MapFile`] type, which can be used to download `.vpk` files from the
//! Workshop. It utilizes an external program called [DepotDownloader] to achieve this.
//!
//! [DepotDownloader]: https://github.com/SteamRE/DepotDownloader

use derive_more::Debug;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::steam::workshop::WorkshopID;
use crate::{Config, Error, Result};

/// A handle to a downloaded map file.
#[derive(Debug)]
pub struct MapFile {
	/// The OS file handle.
	#[debug(skip)]
	file: File,
}

impl MapFile {
	/// Downloads a map from the workshop.
	#[tracing::instrument(level = "debug", skip(api_config), fields(
		file.path = tracing::field::Empty
	))]
	pub async fn download(workshop_id: WorkshopID, api_config: &Config) -> Result<Self> {
		#[cfg(not(feature = "production"))]
		let out_dir = api_config
			.workshop_artifacts_path
			.as_deref()
			.ok_or_else(|| Error::missing_workshop_asset_dir())?;

		#[cfg(feature = "production")]
		let out_dir = &api_config.workshop_artifacts_path;

		#[cfg(not(feature = "production"))]
		let depot_downloader_path = api_config
			.depot_downloader_path
			.as_deref()
			.ok_or_else(|| Error::missing_depot_downloader())?;

		#[cfg(feature = "production")]
		let depot_downloader_path = &api_config.depot_downloader_path;

		let output = Command::new(depot_downloader_path)
			.args(["-app", "730", "-pubfile"])
			.arg(workshop_id.to_string())
			.arg("-dir")
			.arg(out_dir)
			.spawn()
			.map_err(|err| Error::depot_downloader(err))?
			.wait_with_output()
			.await
			.map_err(|err| Error::depot_downloader(err))?;

		let mut stdout = io::stdout();
		let mut stderr = io::stderr();

		if let Err(error) = tokio::try_join!(stdout.flush(), stderr.flush()) {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				%error,
				"failed to flush stdout/stderr",
			};
		}

		if !output.status.success() {
			return Err(Error::depot_downloader(io::Error::new(
				io::ErrorKind::Other,
				"DepotDownloader did not exit successfully",
			)));
		}

		let filepath = out_dir.join(format!("{workshop_id}.vpk"));

		tracing::Span::current().record("file.path", format_args!("{filepath:?}"));

		let file = File::open(&filepath).await.map_err(|err| {
			Error::open_map_file(err)
				.context("failed to open map file")
				.context(format!("path: `{filepath:?}`"))
		})?;

		Ok(Self { file })
	}

	/// Computes the crc32 checksum for this file.
	#[tracing::instrument(level = "debug", skip(self), ret)]
	pub async fn checksum(mut self) -> io::Result<u32> {
		let metadata = self.file.metadata().await?;
		let filesize = usize::try_from(metadata.len()).expect("64-bit platform");
		let mut buf = Vec::with_capacity(filesize);

		self.file.read_to_end(&mut buf).await?;

		Ok(crc32fast::hash(&buf))
	}
}
