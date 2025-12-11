//! Announcements broadcasted to players when they join servers.

use std::num::NonZero;

use crate::context::Context;
use crate::database;
use crate::time::Timestamp;

define_id_type! {
    /// A unique identifier for in-game announcements.
    #[cfg_attr(feature = "fake", derive(fake::Dummy))]
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct AnnouncementId(NonZero<u64>);
}

/// An in-game announcement.
#[derive(Debug, serde::Serialize)]
pub struct Announcement {
    pub id: AnnouncementId,
    pub title: String,
    pub body: String,
    #[serde(serialize_with = "serialize_timestamp")]
    pub created_at: Timestamp,
    #[serde(serialize_with = "serialize_timestamp")]
    pub starts_at: Timestamp,
    #[serde(serialize_with = "serialize_timestamp")]
    pub expires_at: Timestamp,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to get announcements")]
#[from(forward)]
pub struct GetAnnouncementsError(database::Error);

#[tracing::instrument(skip(cx), err(level = "debug"))]
pub async fn get_announcements(cx: &Context) -> Result<Vec<Announcement>, GetAnnouncementsError> {
    sqlx::query_as!(
        Announcement,
        "SELECT
           id `id: AnnouncementId`,
           title,
           body,
           created_at,
           starts_at,
           expires_at
         FROM Announcements
         WHERE expires_at > NOW()
         ORDER BY expires_at ASC",
    )
    .fetch_all(cx.database().as_ref())
    .await
    .map_err(GetAnnouncementsError::from)
}

fn serialize_timestamp<S>(timestamp: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(timestamp.to_unix_ms())
}
