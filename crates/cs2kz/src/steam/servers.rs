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

static INFOS: LazyLock<RwLock<HashMap<ServerId, ServerInfo>>> = LazyLock::new(Default::default);

#[derive(Debug)]
pub struct ServerInfo {
    pub a2s: a2s::info::Info,
    pub map_info: Option<MapInfo>,
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

                (row.id, info)
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
            if let Ok((server_id, a2s_info)) = join_result {
                if let Ok(a2s_info) = a2s_info {
                    let map_info = maps.get(&a2s_info.map).map(|map| MapInfo {
                        workshop_id: map.workshop_id,
                        state: map.state,
                    });

                    INFOS
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(server_id, ServerInfo { a2s: a2s_info, map_info });
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
