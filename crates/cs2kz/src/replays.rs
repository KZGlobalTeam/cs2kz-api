use futures_util::TryFutureExt as _;
use tokio::time::Duration;
use uuid::Uuid;

use crate::time::Timestamp;
use crate::{Context, database};

pub mod cleaner;

define_id_type! {
    pub struct RecordId(Uuid);
}

crate::database::impl_traits!(RecordId as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        let bytes = self.0.as_bytes();
        out = &bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        uuid::Bytes::try_from(bytes)
            .map(Uuid::from_bytes)
            .map(Self)
            .map_err(Into::into)
    }
});

define_id_type! {
    pub struct ReplayUploadKey(Uuid);
}

impl ReplayUploadKey {
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

crate::database::impl_traits!(ReplayUploadKey as [u8] => {
    fn encode<'a>(self, out: &'a [u8]) {
        let bytes = self.0.as_bytes();
        out = &bytes[..];
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, BoxError> {
        uuid::Bytes::try_from(bytes)
            .map(Uuid::from_bytes)
            .map(Self)
            .map_err(Into::into)
    }
});

pub async fn create_upload_key(
    cx: &Context,
    record_id: impl Into<Uuid>,
    ttl: Duration,
) -> Result<ReplayUploadKey, database::Error> {
    let key = ReplayUploadKey::new();

    sqlx::query!(
        "INSERT INTO ReplayUploadKeys VALUES (?, ?, ?)",
        key,
        RecordId(record_id.into()),
        Timestamp::now() + ttl
    )
    .execute(cx.database().as_ref())
    .await?;

    Ok(key)
}

pub async fn claim_upload_key<T, E>(
    cx: &Context,
    key: ReplayUploadKey,
    f: impl AsyncFnOnce(Uuid) -> Result<T, E>,
) -> Result<Option<T>, E>
where
    E: From<database::Error>,
{
    cx.database_transaction(async |conn| {
        let Some(row) = sqlx::query!(
            "SELECT
               record_id `record_id: RecordId`,
               expires_at `expires_at: Timestamp`
             FROM ReplayUploadKeys
             WHERE upload_key = ?
             FOR UPDATE",
            key,
        )
        .fetch_optional(&mut *conn)
        .map_err(database::Error::from)
        .await?
        else {
            return Ok(None);
        };

        if row.expires_at <= Timestamp::now() {
            return Ok(None);
        }

        let result = f(row.record_id.0).await?;

        sqlx::query!("DELETE FROM ReplayUploadKeys WHERE upload_key = ?", key)
            .execute(&mut *conn)
            .map_err(database::Error::from)
            .await?;

        Ok(Some(result))
    })
    .await
}

pub async fn claim_upload_key_by_record_id(
    cx: &Context,
    record_id: impl Into<Uuid>,
) -> Result<bool, database::Error> {
    cx.database_transaction(async |conn| {
        let Some(row) = sqlx::query!(
            "SELECT
               record_id `record_id: RecordId`,
               upload_key `upload_key: ReplayUploadKey`,
               expires_at `expires_at: Timestamp`
             FROM ReplayUploadKeys
             WHERE record_id = ?
             FOR UPDATE",
            RecordId(record_id.into()),
        )
        .fetch_optional(&mut *conn)
        .map_err(database::Error::from)
        .await?
        else {
            return Ok(false);
        };

        if row.expires_at <= Timestamp::now() {
            return Ok(false);
        }

        sqlx::query!("DELETE FROM ReplayUploadKeys WHERE upload_key = ?", row.upload_key)
            .execute(&mut *conn)
            .map_err(database::Error::from)
            .await?;

        Ok(true)
    })
    .await
}
