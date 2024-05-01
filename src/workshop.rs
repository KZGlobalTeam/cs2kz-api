//! Helper types for interacting with Steam's Workshop.

use std::result::Result as StdResult;

use reqwest::header;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::error;

use crate::maps::WorkshopID;
use crate::{Error, Result};

/// A Steam Workshop map that has been downloaded to disk.
#[derive(Debug)]
pub struct WorkshopMap {
	/// File handle to the `.vpk` file.
	file: File,
}

impl WorkshopMap {
	/// Steam WebAPI URL for fetching information about workshop maps.
	const API_URL: &'static str =
		"https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

	/// Fetch the name of the workshop map with the given `workshop_id`.
	pub async fn fetch_name(
		workshop_id: WorkshopID,
		http_client: &reqwest::Client,
	) -> Result<String> {
		let query_params =
			serde_urlencoded::to_string(Params { workshop_id }).expect("this is valid");

		let response = http_client
			.post(Self::API_URL)
			.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
			.body(query_params)
			.send()
			.await?;

		if !response.status().is_success() {
			return Err(Error::unknown("workshop ID"));
		}

		response
			.json::<JsonValue>()
			.await
			.map(serde_json::from_value::<MapInfo>)?
			.map(|map| map.title)
			.map_err(|err| Error::unknown("workshop ID").with_source(err))
	}

	/// Download the workshop map with the given `workshop_id` to disk.
	pub async fn download(workshop_id: WorkshopID, config: &crate::Config) -> Result<Self> {
		/// We return the same error for a bunch of different failure cases.
		///
		/// Each one is logged so we can tell what happened, but we don't need to include
		/// more information in the response for the user.
		#[track_caller]
		fn error() -> Error {
			Error::internal_server_error("cannot download workshop assets")
		}

		let artifacts_path = config.workshop_artifacts_path.as_deref().ok_or_else(|| {
			error!(target: "audit_log", "missing workshop asset directory");
			error()
		})?;

		let downloader_path = config.depot_downloader_path.as_deref().ok_or_else(|| {
			error!(target: "audit_log", "missing DepotDownloader");
			error()
		})?;

		let output = Command::new(downloader_path)
			.args(["-app", "730", "-pubfile"])
			.arg(workshop_id.to_string())
			.arg("-dir")
			.arg(artifacts_path)
			.spawn()
			.map_err(|err| {
				error!(target: "audit_log", %err, "failed to run DepotDownloader");
				error().with_source(err)
			})?
			.wait_with_output()
			.await
			.map_err(|err| {
				error!(target: "audit_log", %err, "failed to wait for DepotDownloader");
				error().with_source(err)
			})?;

		let mut stdout = io::stdout();
		let mut stderr = io::stderr();

		if let Err(err) = tokio::try_join!(stdout.flush(), stderr.flush()) {
			error!(target: "audit_log", %err, "failed to flush stdout/stderr");
		}

		if !output.status.success() {
			error!(target: "audit_log", ?output, "DepotDownloader did not complete successfully");
			return Err(error());
		}

		let path = artifacts_path.join(format!("{workshop_id}.vpk"));
		let file = File::open(&path).await.map_err(|err| {
			error!(target: "audit_log", %err, path = %path.display(), "failed to open map file");
			error().with_source(err)
		})?;

		Ok(Self { file })
	}

	/// Calculate the checksum for this map file.
	///
	/// The algorithm used is [crc32].
	///
	/// [crc32]: crc32fast
	pub async fn checksum(&mut self) -> Result<u32> {
		let mut buf = Vec::new();

		self.file.read_to_end(&mut buf).await.map_err(|err| {
			error!(target: "audit_log", %err, "failed to read map file");
			Error::internal_server_error("failed to calculate checksum for workshop map")
				.with_source(err)
		})?;

		Ok(crc32fast::hash(&buf))
	}
}

/// Query parameters for fetching workshop map information.
struct Params {
	/// The ID of the workshop map.
	workshop_id: WorkshopID,
}

impl Serialize for Params {
	fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_struct("params", 2)?;

		serializer.serialize_field("itemcount", &1)?;
		serializer.serialize_field("publishedfileids[0]", &self.workshop_id)?;
		serializer.end()
	}
}

/// Information about a workshop map.
struct MapInfo {
	/// The map's name.
	title: String,
}

impl<'de> Deserialize<'de> for MapInfo {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error as E;

		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			publishedfiledetails: Vec<JsonValue>,
		}

		Helper1::deserialize(deserializer)
			.map(|x| x.response)
			.map(|mut x| x.publishedfiledetails.remove(0))
			.map(|mut json| json["title"].take())
			.map(|json| json.as_str().map(ToOwned::to_owned))?
			.map(|title| Self { title })
			.ok_or_else(|| E::missing_field("title"))
	}
}
