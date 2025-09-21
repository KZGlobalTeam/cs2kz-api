use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use std::{fmt, io};

use cs2kz::checksum::Checksum;
use cs2kz::steam::WorkshopId;
use futures_util::stream::{self, StreamExt};
use serde::ser::{Serialize, SerializeMap, Serializer};
use tokio::process::Command;
use tokio::task;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::time::FutureExt;
use tracing::Instrument;

use crate::steam;

/// Steam Web API URL for fetching map information.
const MAP_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

#[tracing::instrument(skip(http_client), ret(level = "debug"), err(level = "debug"))]
pub async fn fetch_map_name(
    http_client: &reqwest::Client,
    workshop_id: WorkshopId,
) -> Result<Option<String>, steam::ApiError> {
    struct Form {
        workshop_id: WorkshopId,
    }

    impl Serialize for Form {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut serializer = serializer.serialize_map(Some(2))?;
            serializer.serialize_entry("itemcount", &1)?;
            serializer.serialize_entry("publishedfileids[0]", &self.workshop_id)?;
            serializer.end()
        }
    }

    steam::request(http_client.post(MAP_URL).form(&Form { workshop_id }))
        .await
        .map(|FetchMapResponse { mut publishedfiledetails }| {
            if publishedfiledetails.is_empty() {
                None
            } else {
                Some(publishedfiledetails.remove(0).title)
            }
        })
}

#[tracing::instrument(err(level = "debug"))]
pub async fn download_map(
    workshop_id: WorkshopId,
    depot_downloader_path: &Path,
    out_dir: &Path,
) -> io::Result<()> {
    debug!(
        target: "cs2kz_api::depot_downloader",
        exe_path = %depot_downloader_path.display(),
        out_dir = %out_dir.display(),
        "spawning DepotDownloader process",
    );

    let mut process = Command::new(depot_downloader_path)
        .args(["-app", "730", "-pubfile"])
        .arg(workshop_id.to_string())
        .arg("-dir")
        .arg(out_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = process.stdout.take().unwrap();
    let stderr = process.stderr.take().unwrap();
    drop(process.stdin.take());

    let output_task = task::spawn({
        let stdout = FramedRead::new(stdout, LinesCodec::new()).map(|result| (result, "stdout"));
        let stderr = FramedRead::new(stderr, LinesCodec::new()).map(|result| (result, "stderr"));
        let mut output = stream::select(stdout, stderr);

        async move {
            while let Some((maybe_line, source)) = output.next().await {
                match maybe_line {
                    Ok(line) => debug!(target: "cs2kz_api::depot_downloader", source, "{line}"),
                    Err(error) => {
                        error!(%error, "failed to read line from DepotDownloader's stdout")
                    },
                }
            }

            info!("DepotDownloader exited");
        }
        .in_current_span()
    });

    if !process.wait().await?.success() {
        let error = "DepotDownloader did not exit successfully";
        error!("{error}");
        return Err(io::Error::other(error));
    }

    if let Err(_) = output_task.timeout(Duration::from_secs(3)).await {
        warn!("DepotDownloader output task did not exit within 3 seconds");
    }

    Ok(())
}

#[tracing::instrument(ret(level = "debug"), err)]
pub async fn compute_checksum<P>(workshop_id: WorkshopId, out_dir: P) -> io::Result<Checksum>
where
    P: AsRef<Path> + fmt::Debug + Send + 'static,
{
    task::spawn_blocking(move || {
        let mut checksum = Checksum::builder();
        let out_dir_entries = fs::read_dir(out_dir.as_ref())
            .inspect_err(|err| error!(error = err as &dyn Error, "failed to read directory"))?;

        for entry in out_dir_entries {
            let entry = entry.inspect_err(|err| {
                error!(error = err as &dyn Error, "failed to read directory entry");
            })?;

            let filename = match entry.file_name().into_string() {
                Ok(name) => name,
                Err(name) => {
                    warn!("entry {name:?} is not valid UTF-8?");
                    continue;
                },
            };

            let Some((prefix, rest)) = filename.split_once('_') else {
                continue;
            };

            let Some((_, "vpk")) = rest.split_once('.') else {
                continue;
            };

            if !prefix
                .parse::<WorkshopId>()
                .is_ok_and(|prefix| prefix == workshop_id)
            {
                continue;
            }

            let path = entry.path();
            let mut file = File::open(&path)
                .inspect_err(|err| error!(error = err as &dyn Error, "failed to open {path:?}"))?;

            checksum
                .read_from(&mut file)
                .inspect_err(|err| error!(error = err as &dyn Error, "failed to read {path:?}"))?;
        }

        Ok(checksum.build())
    })
    .await
    .expect("task does not panic")
}

#[derive(Debug, serde::Deserialize)]
struct FetchMapResponse {
    publishedfiledetails: Vec<PublishedFileDetails>,
}

#[derive(Debug, serde::Deserialize)]
struct PublishedFileDetails {
    title: String,
}
