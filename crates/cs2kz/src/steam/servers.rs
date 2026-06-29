use std::collections::HashMap;
use std::error::Error;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use a2s::A2SClient;
use futures_util::{StreamExt as _, TryFutureExt as _};
use tokio::task::JoinSet;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::Context;
use crate::maps::{GetMapsParams, MapState};
use crate::pagination::{Limit, Offset, Paginated};
use crate::servers::ServerId;
use crate::steam::WorkshopId;
use crate::time::Timestamp;

static INFOS: LazyLock<RwLock<HashMap<ServerId, ServerInfo>>> = LazyLock::new(Default::default);

#[derive(Debug)]
pub struct ServerInfo {
    pub a2s: a2s::info::Info,
    pub geo_info: Option<GeoInfo>,
    pub map_info: Option<MapInfo>,
    pub updated_at: Timestamp,
}

#[derive(Debug)]
pub struct GeoInfo {
    pub country_code: String,
    pub region: String,
}

#[derive(Debug)]
pub struct MapInfo {
    pub workshop_id: WorkshopId,
    pub state: MapState,
}

pub fn with_info<R>(server_id: ServerId, f: impl FnOnce(&ServerInfo) -> R) -> Option<R> {
    INFOS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(&server_id)
        .map(f)
}

#[tracing::instrument(skip_all)]
pub async fn periodically_query_servers(cx: Context, cancellation_token: CancellationToken) {
    let mut interval = interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while cancellation_token
        .run_until_cancelled(interval.tick())
        .await
        .is_some()
    {
        let mut addrs = sqlx::query!("SELECT id `id: ServerId`, host, port FROM Servers")
            .fetch(cx.database().as_ref());

        let mut tasks = JoinSet::new();

        while let Some(row) = addrs.next().await {
            let Ok(row) = row else {
                continue;
            };

            tasks.spawn(async move {
                let addr = (row.host.as_str(), row.port);
                let info = A2SClient::new()
                    .and_then(async |client| client.info(addr).await)
                    .await;

                (row.id, row.host, info)
            });
        }

        let maps = crate::maps::get(&cx, GetMapsParams {
            workshop_id: None,
            name: None,
            state: None,
            limit: Limit::new(1000),
            offset: Offset::default(),
        })
        .map_ok(|paginated| paginated.map(|map| (map.name.clone(), map)))
        .and_then(Paginated::collect::<HashMap<_, _>>)
        .map_ok(Paginated::into_inner)
        .await;

        let Ok(maps) = maps else {
            tracing::error!(error = &maps.unwrap_err() as &dyn Error, "failed to fetch maps");
            continue;
        };

        while let Some(join_result) = tasks.join_next().await {
            if let Ok((server_id, host, a2s_info)) = join_result {
                if let Ok(a2s_info) = a2s_info {
                    let map_info = maps.get(&a2s_info.map).map(|map| MapInfo {
                        workshop_id: map.workshop_id,
                        state: map.state,
                    });

                    let geoiplookup_output = tokio::process::Command::new("geoiplookup")
                        .arg(&host)
                        .output()
                        .await
                        .inspect_err(|error| error!(%error, "failed to lookup ip for {host:?}"))
                        .ok();

                    let geo_info = geoiplookup_output.and_then(|output| {
                        if !output.status.success() {
                            return None;
                        }

                        // GeoIP Country Edition: FI, Finland
                        // GeoIP City Edition, Rev 1: FI, 18, N/A, Helsinki, 00191, 60.179699, 24.934401, 0, 0
                        // GeoIP ASNum Edition: AS24940 Hetzner Online GmbH

                        let stdout = str::from_utf8(&output.stdout).ok()?;
                        let mut parts = stdout.lines();

                        let country_line = parts.next().unwrap();
                        let (_, rest) = country_line.split_once("GeoIP Country Edition: ").unwrap();
                        let (country_code, _) = rest.split_once(", ").unwrap();

                        let city_line = parts.next().unwrap();
                        let city = city_line.split(", ").nth(4).unwrap();

                        Some(GeoInfo {
                            country_code: country_code.into(),
                            region: city.into(),
                        })
                    });

                    INFOS
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(server_id, ServerInfo {
                            a2s: a2s_info,
                            geo_info,
                            map_info,
                            updated_at: Timestamp::now(),
                        });
                } else {
                    INFOS
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&server_id);
                }
            }
        }
    }
}
