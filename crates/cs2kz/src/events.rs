use std::future;
use std::sync::{Arc, LazyLock};

use futures_util::{Stream, StreamExt};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::checksum::Checksum;
use crate::maps::courses::CourseFilterId;
use crate::maps::{MapState, NewCourse};
use crate::players::PlayerId;
use crate::plugin::PluginVersionId;
use crate::records::{StylesForNewRecord, SubmittedPB};
use crate::servers::ServerId;
use crate::steam::WorkshopId;
use crate::time::Seconds;

static QUEUE: LazyLock<broadcast::Sender<Arc<Event>>> = LazyLock::new(|| broadcast::channel(16).0);

#[derive(Debug)]
pub enum Event {
    /// A new map has been approved.
    NewMap {
        workshop_id: WorkshopId,
        name: String,
        description: Option<String>,
        state: MapState,
        vpk_checksum: Checksum,
        mappers: Box<[PlayerId]>,
        courses: Box<[NewCourse]>,
    },

    /// A new record has been submitted.
    NewRecord {
        player_id: PlayerId,
        server_id: ServerId,
        filter_id: CourseFilterId,
        styles: StylesForNewRecord,
        teleports: u32,
        time: Seconds,
        plugin_version_id: PluginVersionId,
        pb_data: Option<SubmittedPB>,
    },
}

/// Dispatches an event to any active subscribers.
///
/// # Return
///
/// The return value is an upper bound on how many subscribers may see this event.
pub(crate) fn dispatch(event: Event) -> usize {
    QUEUE.send(Arc::new(event)).ok().unwrap_or(0)
}

/// Returns a [`Stream`] yielding [`Event`]s that were dispatched by this crate.
pub fn subscribe() -> impl Stream<Item = Arc<Event>> {
    BroadcastStream::new(QUEUE.subscribe()).filter_map(|item| {
        future::ready(match item {
            Ok(item) => Some(item),
            Err(BroadcastStreamRecvError::Lagged(n)) => {
                warn!(n, "event queue subscriber lagged");
                None
            },
        })
    })
}
