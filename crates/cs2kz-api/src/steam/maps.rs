use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use std::{cmp, fmt, io};

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
    let out_dir = out_dir.join(workshop_id.to_string());

    debug!(
        target: "cs2kz_api::depot_downloader",
        exe_path = %depot_downloader_path.display(),
        out_dir = %out_dir.display(),
        "spawning DepotDownloader process",
    );

    tokio::fs::create_dir_all(&out_dir).await?;

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
        let mut out_dir_entries = fs::read_dir(out_dir.as_ref())
            .inspect_err(|err| error!(error = err as &dyn Error, "failed to read directory"))?
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        error!(error = &err as &dyn Error, "failed to read directory entry");
                        return Some(Err(err));
                    },
                };

                let filename = entry
                    .file_name()
                    .into_string()
                    .inspect_err(|name| warn!("entry {name:?} is not valid UTF-8?"))
                    .ok()?;

                let (prefix, rest) = filename.split_once('_')?;

                let (_, "vpk") = rest.split_once('.')? else {
                    return None;
                };

                if !prefix
                    .parse::<WorkshopId>()
                    .is_ok_and(|prefix| prefix == workshop_id)
                {
                    return None;
                }

                Some(Ok(filename))
            })
            .collect::<Result<Vec<_>, _>>()?;

        out_dir_entries.sort_by(cmp_filenames);

        let mut checksum = Checksum::builder();

        for entry in out_dir_entries {
            let mut file = File::open(out_dir.as_ref().join(&entry))
                .inspect_err(|err| error!(error = err as &dyn Error, "failed to open {entry:?}"))?;

            checksum
                .read_from(&mut file)
                .inspect_err(|err| error!(error = err as &dyn Error, "failed to read {entry:?}"))?;
        }

        Ok(checksum.build())
    })
    .await
    .expect("task does not panic")
}

fn cmp_filenames(a: &(impl ?Sized + AsRef<str>), b: &(impl ?Sized + AsRef<str>)) -> cmp::Ordering {
    let a = a.as_ref();
    let b = b.as_ref();

    match (a.ends_with("_dir.vpk"), b.ends_with("_dir.vpk")) {
        (true, true) | (false, false) => a.cmp(b),
        (true, false) => cmp::Ordering::Less,
        (false, true) => cmp::Ordering::Greater,
    }
}

#[test]
fn test_filename_sorting() {
    let mut filenames = vec!["foo.vpk"];
    filenames.sort_by(cmp_filenames);
    assert_eq!(filenames, vec!["foo.vpk"]);

    let mut filenames = vec![
        "foo_003.vpk",
        "foo_dir.vpk",
        "foo_002.vpk",
        "foo_001.vpk",
    ];
    filenames.sort_by(cmp_filenames);
    assert_eq!(filenames, vec!["foo_dir.vpk", "foo_001.vpk", "foo_002.vpk", "foo_003.vpk"]);
}

#[derive(Debug, serde::Deserialize)]
struct FetchMapResponse {
    publishedfiledetails: Vec<PublishedFileDetails>,
}

#[derive(Debug, serde::Deserialize)]
struct PublishedFileDetails {
    title: String,
}
