use std::collections::hash_map::{self, HashMap};
use std::sync::{LazyLock, Mutex};

use tokio::time::{Duration, interval};
use uuid::Uuid;

use crate::records::RecordId;
use crate::time::Timestamp;

pub mod cleaner;

type UploadKeys = HashMap<Uuid, PendingReplayUploadInfo>;

static UPLOAD_KEYS: LazyLock<Mutex<UploadKeys>> = LazyLock::new(|| {
    tokio::spawn(async {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            with_upload_keys(purge_expired_keys);
        }
    });
    Default::default()
});

#[derive(Debug)]
struct PendingReplayUploadInfo {
    record_id: RecordId,
    expires_at: Timestamp,
}

fn with_upload_keys<R>(f: impl FnOnce(&mut UploadKeys) -> R) -> R {
    f(&mut UPLOAD_KEYS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

fn purge_expired_keys(keys: &mut UploadKeys) {
    keys.retain(|_, info| info.expires_at > Timestamp::now());
}

pub fn create_upload_key(record_id: RecordId, ttl: Duration) -> Uuid {
    let key = Uuid::new_v4();
    with_upload_keys(|keys| {
        keys.insert(key, PendingReplayUploadInfo { record_id, expires_at: Timestamp::now() + ttl });
    });
    key
}

pub fn claim_upload_key(key: Uuid) -> Option<RecordId> {
    with_upload_keys(|keys| {
        if let hash_map::Entry::Occupied(entry) = keys.entry(key)
            && entry.get().expires_at > Timestamp::now()
        {
            Some(entry.remove().record_id)
        } else {
            None
        }
    })
}
