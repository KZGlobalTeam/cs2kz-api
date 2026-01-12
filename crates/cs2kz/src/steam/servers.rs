use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use a2s::A2SClient;
use a2s::info::Info as ServerInfo;
use futures_util::{StreamExt as _, TryFutureExt as _};
use tokio::task::JoinSet;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::Context;
use crate::servers::ServerId;

static INFOS: LazyLock<RwLock<HashMap<ServerId, ServerInfo>>> = LazyLock::new(Default::default);

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

        while let Some(join_result) = tasks.join_next().await {
            if let Ok((server_id, server_info)) = join_result {
                if let Ok(info) = server_info {
                    INFOS
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(server_id, info);
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
